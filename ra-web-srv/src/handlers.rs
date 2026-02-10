//! HTTP endpoint handlers

use crate::attestation::{att_doc_fmt, make_attestation_docs};
use crate::cipher::CipherMapper;
use crate::crypto::vrf_verify;
use crate::hashing::hash_file;
use crate::nsm::{get_attestation_doc, get_nsm_description, get_randomness_sequence, LocalNsmDigest};
use crate::requests::*;
use crate::state::{AppCache, AppState, AttProofData, AttUserData, ServerState};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Json;
use aws_nitro_enclaves_cose::crypto::openssl::Openssl;
use aws_nitro_enclaves_cose::CoseSign1;
use aws_nitro_enclaves_nsm_api::api::AttestationDoc;
use openssl::asn1::Asn1Time;
use openssl::pkey::PKey;
use openssl::stack::Stack;
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::{X509, X509StoreContext};
use parking_lot::RwLock;
use serde_bytes::ByteBuf;
use serde_json::json;
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::path::Path as StdPath;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info};

/// Generate attestation documents for files at a path
pub async fn generate_handler(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<GenerateRequest>,
) -> impl IntoResponse {
    let path_str = payload.path.clone();
    let path = StdPath::new(&path_str);

    info!("Generate request for path: {}", path_str);

    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(e) => {
            error!("Path not found '{}': {}", path_str, e);
            return (StatusCode::NOT_FOUND, json!({"error": format!("Path not found: {}", e)}).to_string());
        }
    };

    let is_dir = metadata.is_dir();
    let state_clone = state.clone();

    let path_str_clone = path_str.clone();
    tokio::spawn(async move {
        let path_buf = StdPath::new(&path_str_clone).to_path_buf();
        if let Err(e) = visit_files_recursively(&path_buf, state_clone).await {
            error!("Error processing path '{}': {:?}", path_buf.display(), e);
        }
    });

    let message = if is_dir {
        "Started processing directory"
    } else {
        "Started processing file"
    };

    (StatusCode::ACCEPTED, json!({"status": message, "path": path_str}).to_string())
}

/// Recursively visit files and generate attestation documents
pub fn visit_files_recursively<'a>(
    path: &'a StdPath,
    state: Arc<ServerState>,
) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + Sync + 'a>> {
    Box::pin(async move {
        if path.is_dir() {
            let mut entries = tokio::fs::read_dir(path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                visit_files_recursively(&entry_path, Arc::clone(&state)).await?;
            }
        } else if path.is_file() {
            let file_path = path.to_string_lossy().to_string();
            let handle = tokio::task::spawn_blocking({
                let file_path = file_path.clone();
                move || hash_file(&file_path)
            });

            state.tasks.lock().await.insert(file_path.clone(), handle);

            let tasks_clone = Arc::clone(&state.tasks);
            let results_clone = Arc::clone(&state.results);
            let file_path_clone = file_path.clone();
            let app_state_clone = Arc::clone(&state.app_state);
            let app_cache_clone = Arc::clone(&state.app_cache);

            tokio::spawn(async move {
                let task_result = {
                    let mut tasks = tasks_clone.lock().await;
                    if let Some(handle) = tasks.get_mut(&file_path_clone) {
                        Some(async { handle.await }.await)
                    } else {
                        None
                    }
                };

                if let Some(result) = task_result {
                    match result {
                        Ok(Ok(hash)) => {
                            let mut results = results_clone.lock().await;
                            results.insert(file_path_clone.clone(), hash.clone());
                            make_attestation_docs(
                                file_path_clone.clone().as_str(),
                                hash.clone().as_slice(),
                                app_state_clone,
                                app_cache_clone,
                            );
                        }
                        Ok(Err(e)) => {
                            error!("Error hashing file '{}': {:?}", file_path_clone, e);
                        }
                        Err(e) => {
                            error!("Task panicked for '{}': {:?}", file_path_clone, e);
                        }
                    }

                    let mut tasks = tasks_clone.lock().await;
                    tasks.remove(&file_path_clone);
                }
            });

            tokio::task::yield_now().await;
        }
        Ok(())
    })
}

/// Echo handler for testing
pub async fn echo(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Echo request: {:?}", query_params);

    let fd = server_state.app_state.read().nsm_fd;
    debug!("NSM fd: {}", fd);

    let file_path = query_params
        .get("path")
        .cloned()
        .unwrap_or_else(|| "./".to_string());
    debug!("File path: {}", file_path);

    let response: Vec<String> = query_params
        .iter()
        .map(|(key, val)| format!("Query Parameter: {}; Value: {};", key, val))
        .collect();

    let json_response = json!({
        "parameters": response,
        "path": file_path,
    });

    (StatusCode::OK, json_response.to_string())
}

/// Hello handler for testing
pub async fn hello(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Hello request: {:?}", query_params);

    let fd = server_state.app_state.read().nsm_fd;
    debug!("NSM fd: {}", fd);

    (StatusCode::OK, Html("<h1>Hello, World!</h1>"))
}

/// Ready handler for checking file processing status
pub async fn ready_handler(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Ready request: {:?}", query_params);

    let file_path = match query_params.get("path") {
        Some(path) if !path.is_empty() => path.to_owned(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                json!({"error": "'path' parameter is missing or empty"}).to_string(),
            );
        }
    };

    debug!("Checking status for: {}", file_path);

    let results = server_state.results.lock().await;
    if results.contains_key(&file_path) {
        let json_value = json!({
            "file_path": file_path,
            "sha3_hash": hex::encode(results.get(&file_path).unwrap_or(&vec![])),
            "status": "Ready",
        });
        return (StatusCode::OK, serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
            error!("Error formatting to JSON: {}", e);
            json!({"error": format!("JSON format error: {}", e)}).to_string()
        }));
    }

    let tasks = server_state.tasks.lock().await;
    if tasks.contains_key(&file_path) {
        let json_value = json!({
            "file_path": file_path,
            "status": "Processing",
        });
        return (StatusCode::PROCESSING, serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
            error!("Error formatting to JSON: {}", e);
            json!({"error": format!("JSON format error: {}", e)}).to_string()
        }));
    }

    let json_value = json!({
        "file_path": file_path,
        "status": "Not found",
    });
    (StatusCode::NOT_FOUND, serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
        error!("Error formatting to JSON: {}", e);
        json!({"error": format!("JSON format error: {}", e)}).to_string()
    }))
}

/// Readiness handler for batch status check
pub async fn readiness(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Readiness request: {:?}", query_params);

    let path = match query_params.get("path") {
        Some(p) if !p.is_empty() => p.to_owned(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                json!({"error": "'path' parameter is missing or empty"}).to_string(),
            );
        }
    };

    let results = server_state.results.lock().await;
    let ready_items: Vec<_> = results
        .iter()
        .filter(|(key, _)| key.contains(path.as_str()))
        .map(|(file_path, hash)| {
            json!({
                "file_path": file_path,
                "sha3_hash": hex::encode(hash),
                "status": "Ready",
            })
        })
        .collect();

    let tasks = server_state.tasks.lock().await;
    let processing_items: Vec<_> = tasks
        .iter()
        .filter(|(key, _)| key.contains(path.as_str()))
        .map(|(file_path, _)| {
            json!({
                "file_path": file_path,
                "status": "Processing",
            })
        })
        .collect();

    let response = json!({
        "path": path,
        "ready": ready_items,
        "processing": processing_items,
    });

    (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
        error!("Error formatting to JSON: {}", e);
        json!({"error": format!("JSON format error: {}", e)}).to_string()
    }))
}

/// Hash handler for retrieving file hash
pub async fn hash_handler(
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Hash request: {:?}", query_params);

    let file_path = match query_params.get("path") {
        Some(p) if !p.is_empty() => p.to_owned(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                json!({"error": "'path' parameter is missing or empty"}).to_string(),
            );
        }
    };

    let results = state.results.lock().await;
    match results.get(&file_path) {
        Some(hash) => {
            let json_value = json!({
                "file_path": file_path,
                "sha3_hash": hex::encode(hash),
            });
            (StatusCode::OK, serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
        None => {
            let tasks = state.tasks.lock().await;
            if tasks.contains_key(&file_path) {
                (StatusCode::ACCEPTED, json!({"status": "Processing", "file_path": file_path}).to_string())
            } else {
                (StatusCode::NOT_FOUND, json!({"status": "Not found", "file_path": file_path}).to_string())
            }
        }
    }
}

/// Proof handler for retrieving VRF proof
pub async fn proof_handler(
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Proof request: {:?}", query_params);

    let file_path = match query_params.get("path") {
        Some(p) if !p.is_empty() => p.to_owned(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                json!({"error": "'path' parameter is missing or empty"}).to_string(),
            );
        }
    };

    let app_cache = state.app_cache.read().clone().att_data;
    match app_cache.get(&file_path) {
        Some(att_data) => {
            let json_value = json!({
                "file_path": att_data.file_path,
                "sha3_hash": att_data.sha3_hash,
                "vrf_proof": att_data.vrf_proof,
                "vrf_cipher_suite": att_data.vrf_cipher_suite.to_string(),
            });
            (StatusCode::OK, serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
        None => {
            let tasks = state.tasks.lock().await;
            if tasks.contains_key(&file_path) {
                (StatusCode::ACCEPTED, json!({"status": "Processing", "file_path": file_path}).to_string())
            } else {
                (StatusCode::NOT_FOUND, json!({"status": "Not found", "file_path": file_path}).to_string())
            }
        }
    }
}

/// Doc handler for retrieving attestation document
pub async fn doc_handler(
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Doc request: {:?}", query_params);

    let file_path = match query_params.get("path") {
        Some(p) if !p.is_empty() => p.to_owned(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                json!({"error": "'path' parameter is missing or empty"}).to_string(),
            );
        }
    };

    let view = query_params
        .get("view")
        .cloned()
        .unwrap_or_else(|| "json_hex".to_string());

    let app_cache = state.app_cache.read().clone().att_data;
    match app_cache.get(&file_path) {
        Some(att_data) => {
            let att_doc_formatted = match att_doc_fmt(att_data.att_doc.as_slice(), view.as_str()) {
                Ok(formatted) => formatted,
                Err(e) => {
                    error!("Failed to format attestation document: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({"error": format!("Failed to format document: {}", e)}).to_string(),
                    );
                }
            };

            let json_value = json!({
                "file_path": att_data.file_path,
                "sha3_hash": att_data.sha3_hash,
                "vrf_proof": att_data.vrf_proof,
                "vrf_cipher_suite": att_data.vrf_cipher_suite.to_string(),
                "att_doc": serde_json::from_str::<serde_json::Value>(&att_doc_formatted).unwrap_or(serde_json::Value::String(att_doc_formatted)),
            });
            (StatusCode::OK, serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
        None => {
            let tasks = state.tasks.lock().await;
            if tasks.contains_key(&file_path) {
                (StatusCode::ACCEPTED, json!({"status": "Processing", "file_path": file_path}).to_string())
            } else {
                (StatusCode::NOT_FOUND, json!({"status": "Not found", "file_path": file_path}).to_string())
            }
        }
    }
}

/// Hashes handler for retrieving all hashes matching a path
pub async fn hashes(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Hashes request: {:?}", query_params);

    let path = match query_params.get("path") {
        Some(p) if !p.is_empty() => p.to_owned(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                json!({"error": "'path' parameter is missing or empty"}).to_string(),
            );
        }
    };

    let hashes = server_state.results.lock().await;
    let results: Vec<_> = hashes
        .iter()
        .filter(|(key, _)| key.contains(path.as_str()))
        .map(|(path, hash)| {
            json!({
                "file_path": path,
                "sha3_hash": hex::encode(hash.as_slice()),
            })
        })
        .collect();

    let response = json!({
        "path": path,
        "hashes": results,
    });

    (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
        error!("Error formatting to JSON: {}", e);
        json!({"error": format!("JSON format error: {}", e)}).to_string()
    }))
}

/// Proofs handler for retrieving all proofs matching a path
pub async fn proofs(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Proofs request: {:?}", query_params);

    let path = match query_params.get("path") {
        Some(p) if !p.is_empty() => p.to_owned(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                json!({"error": "'path' parameter is missing or empty"}).to_string(),
            );
        }
    };

    let app_cache = server_state.app_cache.read();
    let results: Vec<_> = app_cache
        .att_data
        .iter()
        .filter(|(key, _)| key.contains(path.as_str()))
        .map(|(_, att_data)| {
            json!({
                "file_path": att_data.file_path,
                "sha3_hash": att_data.sha3_hash,
                "vrf_proof": att_data.vrf_proof,
                "vrf_cipher_suite": att_data.vrf_cipher_suite.to_string(),
            })
        })
        .collect();

    let response = json!({
        "path": path,
        "proofs": results,
    });

    (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
        error!("Error formatting to JSON: {}", e);
        json!({"error": format!("JSON format error: {}", e)}).to_string()
    }))
}

/// Docs handler for retrieving all attestation documents matching a path
pub async fn docs(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Docs request: {:?}", query_params);

    let path = match query_params.get("path") {
        Some(p) if !p.is_empty() => p.to_owned(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                json!({"error": "'path' parameter is missing or empty"}).to_string(),
            );
        }
    };

    let view = query_params
        .get("view")
        .cloned()
        .unwrap_or_else(|| "json_hex".to_string());

    let app_cache = server_state.app_cache.read();
    let results: Vec<_> = app_cache
        .att_data
        .iter()
        .filter(|(key, _)| key.contains(path.as_str()))
        .filter_map(|(_, att_data)| {
            let att_doc_formatted = att_doc_fmt(att_data.att_doc.as_slice(), view.as_str()).ok()?;
            Some(json!({
                "file_path": att_data.file_path,
                "sha3_hash": att_data.sha3_hash,
                "vrf_proof": att_data.vrf_proof,
                "vrf_cipher_suite": att_data.vrf_cipher_suite.to_string(),
                "att_doc": serde_json::from_str::<serde_json::Value>(&att_doc_formatted).unwrap_or(serde_json::Value::String(att_doc_formatted)),
            }))
        })
        .collect();

    let response = json!({
        "path": path,
        "documents": results,
    });

    (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
        error!("Error formatting to JSON: {}", e);
        json!({"error": format!("JSON format error: {}", e)}).to_string()
    }))
}

/// Pubkeys handler for retrieving public keys
pub async fn pubkeys(
    Query(query_params): Query<HashMap<String, String>>,
    State(app_state): State<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    debug!("Pubkeys request: {:?}", query_params);

    let view = query_params
        .get("view")
        .cloned()
        .unwrap_or_else(|| "hex".to_string());

    let fmt = query_params
        .get("fmt")
        .cloned()
        .unwrap_or_else(|| "pem".to_string());

    let app_state = app_state.read().clone();
    let cipher = app_state.vrf_cipher_suite.to_nid();

    // Process proofs key
    let skey4proofs_bytes = app_state.sk4proofs;
    let skey4proofs_pubkey_result = (|| -> Result<Vec<u8>, String> {
        let skey4proofs_pkey = PKey::private_key_from_pem(skey4proofs_bytes.as_slice())
            .map_err(|e| format!("Failed to parse proofs private key: {}", e))?;
        let skey4proofs_eckey = skey4proofs_pkey.ec_key()
            .map_err(|e| format!("Failed to get EC key for proofs: {}", e))?;

        let alg = openssl::ec::EcGroup::from_curve_name(cipher)
            .map_err(|e| format!("Failed to get EC group: {}", e))?;
        let skey4proofs_ec_pubkey = openssl::ec::EcKey::from_public_key(&alg, skey4proofs_eckey.public_key())
            .map_err(|e| format!("Failed to create public key: {}", e))?;
        let skey4proofs_pkey_pubkey = PKey::from_ec_key(skey4proofs_ec_pubkey)
            .map_err(|e| format!("Failed to convert to PKey: {}", e))?;

        match fmt.as_str() {
            "der" => skey4proofs_pkey_pubkey.public_key_to_der()
                .map_err(|e| format!("Failed to convert to DER: {}", e)),
            _ => skey4proofs_pkey_pubkey.public_key_to_pem()
                .map_err(|e| format!("Failed to convert to PEM: {}", e)),
        }
    })();

    // Process docs key
    let skey4docs_bytes = app_state.sk4docs;
    let skey4docs_pubkey_result = (|| -> Result<Vec<u8>, String> {
        let skey4docs_pkey = PKey::private_key_from_pem(skey4docs_bytes.as_slice())
            .map_err(|e| format!("Failed to parse docs private key: {}", e))?;
        let skey4docs_eckey = skey4docs_pkey.ec_key()
            .map_err(|e| format!("Failed to get EC key for docs: {}", e))?;

        let alg = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::SECP521R1)
            .map_err(|e| format!("Failed to get EC group: {}", e))?;
        let skey4docs_ec_pubkey = openssl::ec::EcKey::from_public_key(&alg, skey4docs_eckey.public_key())
            .map_err(|e| format!("Failed to create public key: {}", e))?;
        let skey4docs_pkey_pubkey = PKey::from_ec_key(skey4docs_ec_pubkey)
            .map_err(|e| format!("Failed to convert to PKey: {}", e))?;

        match fmt.as_str() {
            "der" => skey4docs_pkey_pubkey.public_key_to_der()
                .map_err(|e| format!("Failed to convert to DER: {}", e)),
            _ => skey4docs_pkey_pubkey.public_key_to_pem()
                .map_err(|e| format!("Failed to convert to PEM: {}", e)),
        }
    })();

    let response = match (skey4proofs_pubkey_result, skey4docs_pubkey_result) {
        (Ok(proofs_pubkey), Ok(docs_pubkey)) => {
            match view.as_str() {
                "string" | "text" => {
                    json!({
                        "pubkey4proofs": String::from_utf8_lossy(&proofs_pubkey).to_string(),
                        "pubkey4docs": String::from_utf8_lossy(&docs_pubkey).to_string(),
                    })
                }
                _ => {
                    json!({
                        "pubkey4proofs": hex::encode(&proofs_pubkey),
                        "pubkey4docs": hex::encode(&docs_pubkey),
                    })
                }
            }
        }
        (Err(e1), Err(e2)) => {
            error!("Failed to get both public keys: {}, {}", e1, e2);
            json!({"error": format!("Failed to get public keys: {}, {}", e1, e2)})
        }
        (Err(e), _) => {
            error!("Failed to get proofs public key: {}", e);
            json!({"error": format!("Failed to get proofs public key: {}", e)})
        }
        (_, Err(e)) => {
            error!("Failed to get docs public key: {}", e);
            json!({"error": format!("Failed to get docs public key: {}", e)})
        }
    };

    (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
        error!("Error formatting to JSON: {}", e);
        json!({"error": format!("JSON format error: {}", e)}).to_string()
    }))
}

/// NSM description handler
pub async fn nsm_desc(
    Query(query_params): Query<HashMap<String, String>>,
    State(app_state): State<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    debug!("NSM desc request: {:?}", query_params);

    let fd = app_state.read().nsm_fd;

    match get_nsm_description(fd) {
        Ok(description) => {
            let response = json!({
                "version_major": description.version_major,
                "version_minor": description.version_minor,
                "version_patch": description.version_patch,
                "module_id": description.module_id,
                "max_pcrs": description.max_pcrs,
                "locked_pcrs": description.locked_pcrs.iter().collect::<Vec<_>>(),
                "digest": LocalNsmDigest(description.digest).to_string(),
            });

            info!(
                "NSM description: module_id={}, version={}.{}.{}, max_pcrs={}",
                description.module_id,
                description.version_major,
                description.version_minor,
                description.version_patch,
                description.max_pcrs
            );

            (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
        Err(e) => {
            error!("Failed to get NSM description: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to get NSM description: {}", e)}).to_string())
        }
    }
}

/// Random sequence handler
pub async fn rng_seq(
    Query(query_params): Query<HashMap<String, String>>,
    State(app_state): State<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    debug!("RNG seq request: {:?}", query_params);

    let fd = app_state.read().nsm_fd;
    let length = query_params
        .get("length")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(512);

    match get_randomness_sequence(fd, length) {
        Ok(sequence) => {
            let response = json!({
                "length": sequence.len(),
                "random_hex": hex::encode(&sequence),
            });
            (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
        Err(e) => {
            error!("Failed to get random sequence: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to get random sequence: {}", e)}).to_string())
        }
    }
}

/// Get PCRs handler
pub async fn get_pcrs(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    debug!("Get PCRs request: {:?}", query_params);

    let fd = server_state.app_state.read().nsm_fd;

    let cose_doc_bytes = match get_attestation_doc(fd, None, None, None) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to get attestation document: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to get attestation document: {}", e)}).to_string());
        }
    };

    let cose_doc = match CoseSign1::from_bytes(cose_doc_bytes.as_slice()) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse COSE document: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to parse COSE document: {:?}", e)}).to_string());
        }
    };

    let (_, attestation_doc_bytes) = match cose_doc.get_protected_and_payload::<Openssl>(None) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to get payload: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to get payload: {:?}", e)}).to_string());
        }
    };

    let attestation_doc = match AttestationDoc::from_binary(&attestation_doc_bytes[..]) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse attestation document: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to parse attestation document: {:?}", e)}).to_string());
        }
    };

    let pcrs: HashMap<String, String> = attestation_doc
        .pcrs
        .iter()
        .map(|(key, val)| (key.to_string(), hex::encode(val.clone().into_vec())))
        .collect();

    let response = json!({
        "description": "Actual (run-time) PCR registers of running enclave",
        "pcrs": pcrs,
    });

    (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
        error!("Error formatting to JSON: {}", e);
        json!({"error": format!("JSON format error: {}", e)}).to_string()
    }))
}

/// Verify PCRs handler
pub async fn verify_pcrs(
    State(app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyPCRsRequest>,
) -> impl IntoResponse {
    debug!("Verify PCRs request: {:?}", payload);

    let fd = app_state.read().nsm_fd;

    let cose_doc_bytes = match get_attestation_doc(fd, None, None, None) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to get attestation document: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to get attestation document: {}", e)}).to_string());
        }
    };

    let cose_doc = match CoseSign1::from_bytes(cose_doc_bytes.as_slice()) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse COSE document: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to parse COSE document: {:?}", e)}).to_string());
        }
    };

    let (_, attestation_doc_bytes) = match cose_doc.get_protected_and_payload::<Openssl>(None) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to get payload: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to get payload: {:?}", e)}).to_string());
        }
    };

    let attestation_doc = match AttestationDoc::from_binary(&attestation_doc_bytes[..]) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse attestation document: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to parse attestation document: {:?}", e)}).to_string());
        }
    };

    let pcrs_fmt = attestation_doc
        .pcrs
        .iter()
        .map(|(key, val)| format!("{}: {}", key, hex::encode(val.clone().into_vec())))
        .collect::<Vec<String>>()
        .join(", ");

    let is_valid = payload.pcrs == pcrs_fmt;

    let response = json!({
        "valid": is_valid,
        "message": if is_valid {
            "PCRs match - enclave integrity verified"
        } else {
            "PCRs do not match - potential integrity issue"
        },
        "provided_pcrs": payload.pcrs,
        "actual_pcrs": pcrs_fmt,
    });

    (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
        error!("Error formatting to JSON: {}", e);
        json!({"error": format!("JSON format error: {}", e)}).to_string()
    }))
}

/// Verify hash handler
pub async fn verify_hash(
    State(_app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyHashRequest>,
) -> impl IntoResponse {
    debug!("Verify hash request: {:?}", payload);

    let path = StdPath::new(&payload.file_path);

    let metadata = match tokio::fs::metadata(path).await {
        Ok(m) => m,
        Err(e) => {
            return (StatusCode::NOT_FOUND, json!({"error": format!("File not found: {}", e)}).to_string());
        }
    };

    if metadata.is_dir() {
        return (StatusCode::BAD_REQUEST, json!({"error": "Path is a directory, expected a file"}).to_string());
    }

    let hash = match hash_file(&payload.file_path) {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to hash file '{}': {}", payload.file_path, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to hash file: {}", e)}).to_string());
        }
    };

    let computed_hash = hex::encode(hash.as_slice());
    let is_valid = computed_hash == payload.sha3_hash;

    let response = json!({
        "valid": is_valid,
        "file_path": payload.file_path,
        "provided_hash": payload.sha3_hash,
        "computed_hash": computed_hash,
        "message": if is_valid {
            "Hash matches - file integrity verified"
        } else {
            "Hash does not match - file may have been modified"
        },
    });

    (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
        error!("Error formatting to JSON: {}", e);
        json!({"error": format!("JSON format error: {}", e)}).to_string()
    }))
}

/// Verify proof handler
pub async fn verify_proof(
    State(_app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyProofRequest>,
) -> impl IntoResponse {
    debug!("Verify proof request");

    let att_doc_user_data_bytes = match hex::decode(&payload.user_data) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to decode user_data: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid user_data hex: {}", e)}).to_string());
        }
    };

    let att_doc_user_data: AttUserData = match serde_json::from_slice(&att_doc_user_data_bytes) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse user_data: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid user_data format: {}", e)}).to_string());
        }
    };

    let pubkey_bytes = match hex::decode(&payload.public_key) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to decode public_key: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid public_key hex: {}", e)}).to_string());
        }
    };

    let vrf_proof_bytes = match hex::decode(&att_doc_user_data.vrf_proof) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to decode vrf_proof: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid vrf_proof hex: {}", e)}).to_string());
        }
    };

    let att_proof_data = AttProofData {
        file_path: att_doc_user_data.file_path.clone(),
        sha3_hash: hex::encode(&att_doc_user_data.sha3_hash),
    };

    let att_proof_data_json_bytes = match serde_json::to_vec(&att_proof_data) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to serialize proof data: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to serialize proof data: {}", e)}).to_string());
        }
    };

    match vrf_verify(
        &att_proof_data_json_bytes,
        &vrf_proof_bytes,
        &pubkey_bytes,
        att_doc_user_data.vrf_cipher_suite,
    ) {
        Ok(message) => {
            let response = json!({
                "valid": true,
                "message": message,
            });
            (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
        Err(message) => {
            let response = json!({
                "valid": false,
                "message": message,
            });
            (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
    }
}

/// Check certificate validity
fn check_cert_validity(cert: &X509) -> Result<bool, String> {
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).map_err(|e| format!("Time error: {}", e))?;
    let seconds_since_epoch = since_epoch.as_secs() as i64;
    let now_asn1 = Asn1Time::from_unix(seconds_since_epoch).map_err(|e| format!("ASN1 time error: {}", e))?;

    let not_before_valid = match cert.not_before().compare(&now_asn1) {
        Ok(ord) => !ord.is_gt(),
        Err(e) => return Err(format!("Failed to compare not_before: {}", e)),
    };

    let not_after_valid = match cert.not_after().compare(&now_asn1) {
        Ok(ord) => !ord.is_lt(),
        Err(e) => return Err(format!("Failed to compare not_after: {}", e)),
    };

    Ok(not_before_valid && not_after_valid)
}

/// Verify attestation document handler
pub async fn verify_doc(
    State(_app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyDocRequest>,
) -> impl IntoResponse {
    debug!("Verify doc request");

    let cose_doc_bytes = match hex::decode(&payload.cose_doc_bytes) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to decode cose_doc_bytes: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid cose_doc_bytes hex: {}", e)}).to_string());
        }
    };

    let cose_doc = match CoseSign1::from_bytes(&cose_doc_bytes) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse COSE document: {:?}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid COSE document: {:?}", e)}).to_string());
        }
    };

    let (_, attestation_doc_bytes) = match cose_doc.get_protected_and_payload::<Openssl>(None) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to get payload: {:?}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Failed to get payload: {:?}", e)}).to_string());
        }
    };

    let attestation_doc = match AttestationDoc::from_binary(&attestation_doc_bytes[..]) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse attestation document: {:?}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid attestation document: {:?}", e)}).to_string());
        }
    };

    let cert_bytes = attestation_doc.certificate.into_vec();
    let cert = match X509::from_der(&cert_bytes) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to parse certificate: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid certificate: {}", e)}).to_string());
        }
    };

    let pubkey = match cert.public_key() {
        Ok(pk) => pk,
        Err(e) => {
            error!("Failed to get certificate public key: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to get public key: {}", e)}).to_string());
        }
    };

    match cose_doc.verify_signature::<Openssl>(&pubkey) {
        Ok(true) => {
            let response = json!({
                "valid": true,
                "message": "Attestation document signature is valid",
            });
            (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
        Ok(false) => {
            let response = json!({
                "valid": false,
                "message": "Attestation document signature is invalid",
            });
            (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
        Err(e) => {
            error!("Signature verification error: {:?}", e);
            (StatusCode::BAD_REQUEST, json!({"error": format!("Verification error: {:?}", e)}).to_string())
        }
    }
}

/// Verify certificate validity handler
pub async fn verify_cert_valid(
    State(_app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyDocRequest>,
) -> impl IntoResponse {
    debug!("Verify cert valid request");

    let cose_doc_bytes = match hex::decode(&payload.cose_doc_bytes) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to decode cose_doc_bytes: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid cose_doc_bytes hex: {}", e)}).to_string());
        }
    };

    let cose_doc = match CoseSign1::from_bytes(&cose_doc_bytes) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse COSE document: {:?}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid COSE document: {:?}", e)}).to_string());
        }
    };

    let (_, attestation_doc_bytes) = match cose_doc.get_protected_and_payload::<Openssl>(None) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to get payload: {:?}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Failed to get payload: {:?}", e)}).to_string());
        }
    };

    let attestation_doc = match AttestationDoc::from_binary(&attestation_doc_bytes[..]) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse attestation document: {:?}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid attestation document: {:?}", e)}).to_string());
        }
    };

    let cert_bytes = attestation_doc.certificate.into_vec();
    let cert = match X509::from_der(&cert_bytes) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to parse certificate: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid certificate: {}", e)}).to_string());
        }
    };

    let cert_info = cert.to_text()
        .map(|b| String::from_utf8_lossy(&b).to_string())
        .unwrap_or_else(|_| "Unable to extract certificate info".to_string());

    // Verify certificate signature
    let pubkey = match cert.public_key() {
        Ok(pk) => pk,
        Err(e) => {
            error!("Failed to get certificate public key: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to get public key: {}", e)}).to_string());
        }
    };

    let signature_valid = match cert.verify(&pubkey) {
        Ok(valid) => valid,
        Err(e) => {
            error!("Certificate signature verification failed: {}", e);
            return (StatusCode::BAD_REQUEST, json!({
                "error": format!("Certificate signature verification failed: {}", e),
                "certificate_info": cert_info,
            }).to_string());
        }
    };

    // Check certificate validity period
    let validity_result = check_cert_validity(&cert);

    let response = match (signature_valid, validity_result) {
        (true, Ok(true)) => json!({
            "signature_valid": true,
            "time_valid": true,
            "message": "Certificate is valid",
            "certificate_info": cert_info,
        }),
        (false, Ok(true)) => json!({
            "signature_valid": false,
            "time_valid": true,
            "message": "Certificate signature is invalid",
            "certificate_info": cert_info,
        }),
        (true, Ok(false)) => json!({
            "signature_valid": true,
            "time_valid": false,
            "message": "Certificate has expired or is not yet valid",
            "certificate_info": cert_info,
        }),
        (false, Ok(false)) => json!({
            "signature_valid": false,
            "time_valid": false,
            "message": "Certificate signature is invalid and has expired or is not yet valid",
            "certificate_info": cert_info,
        }),
        (sig_valid, Err(e)) => json!({
            "signature_valid": sig_valid,
            "time_valid": false,
            "error": format!("Validity check error: {}", e),
            "certificate_info": cert_info,
        }),
    };

    (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
        json!({"error": format!("JSON format error: {}", e)}).to_string()
    }))
}

/// Verify certificate bundle handler
pub async fn verify_cert_bundle(
    State(_app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyDocRequest>,
) -> impl IntoResponse {
    debug!("Verify cert bundle request");

    let cose_doc_bytes = match hex::decode(&payload.cose_doc_bytes) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to decode cose_doc_bytes: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid cose_doc_bytes hex: {}", e)}).to_string());
        }
    };

    let cose_doc = match CoseSign1::from_bytes(&cose_doc_bytes) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse COSE document: {:?}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid COSE document: {:?}", e)}).to_string());
        }
    };

    let (_, attestation_doc_bytes) = match cose_doc.get_protected_and_payload::<Openssl>(None) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to get payload: {:?}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Failed to get payload: {:?}", e)}).to_string());
        }
    };

    let attestation_doc = match AttestationDoc::from_binary(&attestation_doc_bytes[..]) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to parse attestation document: {:?}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid attestation document: {:?}", e)}).to_string());
        }
    };

    if attestation_doc.cabundle.is_empty() {
        error!("CA bundle is empty");
        return (StatusCode::BAD_REQUEST, json!({"error": "CA bundle is empty"}).to_string());
    }

    // Parse end-entity certificate
    let cert_bytes = attestation_doc.certificate.into_vec();
    let end_cert = match X509::from_der(&cert_bytes) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to parse end-entity certificate: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid certificate: {}", e)}).to_string());
        }
    };

    // Check end-entity certificate validity
    if let Err(e) = check_cert_validity(&end_cert) {
        error!("End-entity certificate validity check failed: {}", e);
        return (StatusCode::BAD_REQUEST, json!({"error": format!("Certificate validity check failed: {}", e)}).to_string());
    }

    // Parse CA bundle
    let (root_cert_bytes, intermediate_certs) = match attestation_doc.cabundle.split_first() {
        Some((root, rest)) => (root.to_vec(), rest),
        None => {
            error!("CA bundle is empty");
            return (StatusCode::BAD_REQUEST, json!({"error": "CA bundle is empty"}).to_string());
        }
    };

    let root_cert = match X509::from_der(&root_cert_bytes) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to parse root certificate: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid root certificate: {}", e)}).to_string());
        }
    };

    // Check root certificate validity
    if let Err(e) = check_cert_validity(&root_cert) {
        error!("Root certificate validity check failed: {}", e);
        return (StatusCode::BAD_REQUEST, json!({"error": format!("Root certificate validity check failed: {}", e)}).to_string());
    }

    // Build certificate store with root certificate
    let mut store_builder = match X509StoreBuilder::new() {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to create certificate store builder: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to create store: {}", e)}).to_string());
        }
    };

    if let Err(e) = store_builder.add_cert(root_cert) {
        error!("Failed to add root certificate to store: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to add root cert: {}", e)}).to_string());
    }

    let store = store_builder.build();

    // Build intermediate certificate chain
    let mut intermediate_stack = match Stack::new() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to create certificate stack: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to create stack: {}", e)}).to_string());
        }
    };

    for cert_bytes in intermediate_certs {
        let cert = match X509::from_der(&cert_bytes.to_vec()) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to parse intermediate certificate: {}", e);
                return (StatusCode::BAD_REQUEST, json!({"error": format!("Invalid intermediate certificate: {}", e)}).to_string());
            }
        };

        if let Err(e) = check_cert_validity(&cert) {
            error!("Intermediate certificate validity check failed: {}", e);
            return (StatusCode::BAD_REQUEST, json!({"error": format!("Intermediate certificate validity check failed: {}", e)}).to_string());
        }

        if let Err(e) = intermediate_stack.push(cert) {
            error!("Failed to add intermediate certificate: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to add intermediate cert: {}", e)}).to_string());
        }
    }

    // Verify certificate chain
    let mut ctx = match X509StoreContext::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create store context: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": format!("Failed to create context: {}", e)}).to_string());
        }
    };

    let verification_result = ctx.init(&store, &end_cert, &intermediate_stack, |ctx| ctx.verify_cert());

    match verification_result {
        Ok(true) => {
            let response = json!({
                "valid": true,
                "message": "Certificate chain verification successful",
            });
            (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
        Ok(false) => {
            let error_string = ctx.error().error_string();
            let response = json!({
                "valid": false,
                "message": "Certificate chain verification failed",
                "error_detail": error_string,
            });
            (StatusCode::OK, serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
                json!({"error": format!("JSON format error: {}", e)}).to_string()
            }))
        }
        Err(e) => {
            error!("Certificate chain verification error: {}", e);
            (StatusCode::BAD_REQUEST, json!({"error": format!("Verification error: {}", e)}).to_string())
        }
    }
}
