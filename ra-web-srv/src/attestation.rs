//! Attestation document handling and formatting

use crate::cipher::CipherMapper;
use crate::crypto::vrf_proof;
use crate::errors::{AppResult, AttestationError};
use crate::nsm::{get_attestation_doc, LocalNsmDigest};
use crate::state::{AppCache, AppState, AttData, AttProofData, AttUserData};
use aws_nitro_enclaves_cose::crypto::openssl::Openssl;
use aws_nitro_enclaves_cose::CoseSign1;
use aws_nitro_enclaves_nsm_api::api::AttestationDoc;
use bytes::Bytes;
use openssl::pkey::PKey;
use parking_lot::RwLock;
use serde_bytes::ByteBuf;
use std::sync::Arc;
use tracing::{debug, error, info};
use vrf::openssl::ECVRF;
use vrf::VRF;

/// Generate attestation documents for a file
pub fn make_attestation_docs(
    file_path: &str,
    hash: &[u8],
    app_state: Arc<RwLock<AppState>>,
    app_cache: Arc<RwLock<AppCache>>,
) {
    let file_path_string = file_path.to_string();
    debug!("Generating attestation documents for: {}", file_path_string);

    let app_state_read = app_state.read().clone();

    // Parse the private key for proofs
    let skey4proofs_bytes = app_state_read.sk4proofs.clone();
    let skey4proofs_pkey = match PKey::private_key_from_pem(skey4proofs_bytes.as_slice()) {
        Ok(pkey) => pkey,
        Err(e) => {
            error!("Failed to parse private key for proofs: {}", e);
            return;
        }
    };

    let skey4proofs_eckey = match skey4proofs_pkey.ec_key() {
        Ok(eckey) => eckey,
        Err(e) => {
            error!("Failed to get EC key for proofs: {}", e);
            return;
        }
    };

    let skey4proofs_bignum = match skey4proofs_eckey.private_key().to_owned() {
        Ok(bignum) => bignum,
        Err(e) => {
            error!("Failed to get private key bignum: {}", e);
            return;
        }
    };

    let skey4proofs_vec = skey4proofs_bignum.to_vec();

    // Generate VRF proof
    let att_proof_data = AttProofData {
        file_path: file_path_string.clone(),
        sha3_hash: hex::encode(hash),
    };

    let att_proof_data_json_bytes = match serde_json::to_vec(&att_proof_data) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to serialize attestation proof data: {}", e);
            return;
        }
    };

    let cipher_suite = app_state_read.vrf_cipher_suite;

    let vrf_proof_result = match vrf_proof(
        att_proof_data_json_bytes.as_slice(),
        skey4proofs_vec.as_slice(),
        cipher_suite.clone(),
    ) {
        Ok(proof) => proof,
        Err(e) => {
            error!("Failed to generate VRF proof: {}", e);
            return;
        }
    };

    // Generate nonce using VRF
    let mut vrf = match ECVRF::from_suite(cipher_suite.clone()) {
        Ok(vrf) => vrf,
        Err(e) => {
            error!("Failed to create VRF suite: {:?}", e);
            return;
        }
    };

    let nonce = match vrf.generate_nonce(&skey4proofs_bignum, att_proof_data_json_bytes.as_slice()) {
        Ok(nonce) => nonce.to_vec(),
        Err(e) => {
            error!("Failed to generate nonce: {:?}", e);
            return;
        }
    };

    let skey4proofs_pubkey = match vrf.derive_public_key(skey4proofs_vec.as_slice()) {
        Ok(pubkey) => pubkey,
        Err(e) => {
            error!("Failed to derive public key: {:?}", e);
            return;
        }
    };

    // Create user data for attestation document
    let att_user_data = AttUserData {
        file_path: file_path_string.clone(),
        sha3_hash: hex::encode(hash),
        vrf_proof: hex::encode(vrf_proof_result.clone()),
        vrf_cipher_suite: cipher_suite.clone(),
    };

    let att_user_data_json_bytes = match serde_json::to_vec(&att_user_data) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to serialize user data: {}", e);
            return;
        }
    };

    // Get attestation document from NSM
    let fd = app_state_read.nsm_fd;
    let att_doc = match get_attestation_doc(
        fd,
        Some(ByteBuf::from(att_user_data_json_bytes)),
        Some(ByteBuf::from(nonce.clone())),
        Some(ByteBuf::from(skey4proofs_pubkey.clone())),
    ) {
        Ok(doc) => doc,
        Err(e) => {
            error!("Failed to get attestation document: {}", e);
            return;
        }
    };

    // Store attestation data
    let att_data = AttData {
        file_path: file_path_string.clone(),
        sha3_hash: hex::encode(hash),
        vrf_proof: hex::encode(vrf_proof_result.clone()),
        vrf_cipher_suite: cipher_suite.clone(),
        att_doc: att_doc.clone(),
    };

    {
        let mut app_cache = app_cache.write();
        app_cache.att_data.insert(file_path_string.clone(), att_data.clone());
    }

    // Store in NATS if available
    if let Some(store) = app_state_read.storage.clone() {
        let att_data_json_bytes = match serde_json::to_vec(&att_data) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to serialize attestation data for NATS: {}", e);
                return;
            }
        };

        let file_path_for_task = file_path_string.clone();
        tokio::task::spawn(async move {
            if let Err(e) = store.put(&file_path_for_task, Bytes::from(att_data_json_bytes)).await {
                error!(
                    "[NATS Producer] Error while putting data to KV store for '{}': {}",
                    file_path_for_task, e
                );
            } else {
                debug!("[NATS Producer] Stored attestation data for: {}", file_path_for_task);
            }
        });
    }

    info!("Attestation documents generated for: {}", file_path_string);
}

/// Format attestation document for output
pub fn att_doc_fmt(att_doc: &[u8], view: &str) -> AppResult<String> {
    debug!("Formatting attestation document with view: {}", view);

    let cose_doc = CoseSign1::from_bytes(att_doc).map_err(|e| {
        error!("Failed to parse COSE document: {:?}", e);
        AttestationError::CoseParseError(format!("{:?}", e))
    })?;

    let (protected_header, attestation_doc_bytes) = cose_doc
        .get_protected_and_payload::<Openssl>(None)
        .map_err(|e| {
            error!("Failed to get protected header and payload: {:?}", e);
            AttestationError::CoseParseError(format!("{:?}", e))
        })?;

    debug!("Protected header: {:?}", protected_header);

    let unprotected_header = cose_doc.get_unprotected();
    debug!("Unprotected header: {:?}", unprotected_header);

    let attestation_doc = AttestationDoc::from_binary(&attestation_doc_bytes[..]).map_err(|e| {
        error!("Failed to parse attestation document: {:?}", e);
        AttestationError::AttDocParseError(format!("{:?}", e))
    })?;

    debug!("Attestation document parsed successfully");

    let attestation_doc_signature = cose_doc.get_signature();
    debug!(
        "Attestation document signature: {}",
        hex::encode(attestation_doc_signature.clone())
    );

    // Parse user data
    let att_doc_user_data_bytes = attestation_doc
        .clone()
        .user_data
        .unwrap_or(ByteBuf::new())
        .into_vec();

    let att_doc_user_data: AttUserData = serde_json::from_slice(att_doc_user_data_bytes.as_slice())
        .unwrap_or_else(|e| {
            error!("Failed to parse user data from attestation document: {}", e);
            AttUserData::default()
        });

    let att_doc_user_data_json_string = serde_json::to_string_pretty(&att_doc_user_data)
        .unwrap_or_else(|e| {
            error!("Failed to serialize user data: {}", e);
            String::new()
        });

    let attestation_doc_json_string = serde_json::to_string_pretty(&attestation_doc)
        .unwrap_or_else(|e| {
            error!("Failed to serialize attestation document: {}", e);
            String::new()
        });

    // Format headers
    let header_protected_str = protected_header
        .into_inner()
        .iter()
        .map(|(key, val)| {
            let key_hex = serde_cbor::to_vec(key)
                .map(|v| hex::encode(v))
                .unwrap_or_else(|_| "error".to_string());
            let val_hex = serde_cbor::to_vec(val)
                .map(|v| hex::encode(v))
                .unwrap_or_else(|_| "error".to_string());
            format!("{}: {}", key_hex, val_hex)
        })
        .collect::<Vec<String>>()
        .join(", ");

    let header_unprotected_str = unprotected_header
        .into_inner()
        .iter()
        .map(|(key, val)| {
            let key_hex = serde_cbor::to_vec(key)
                .map(|v| hex::encode(v))
                .unwrap_or_else(|_| "error".to_string());
            let val_hex = serde_cbor::to_vec(val)
                .map(|v| hex::encode(v))
                .unwrap_or_else(|_| "error".to_string());
            format!("{}: {}", key_hex, val_hex)
        })
        .collect::<Vec<String>>()
        .join(", ");

    // Format PCRs
    let pcrs_fmt = attestation_doc
        .pcrs
        .iter()
        .map(|(key, val)| format!("{}: {}", key, hex::encode(val.clone().into_vec())))
        .collect::<Vec<String>>()
        .join(", ");

    // Format CA bundle
    let cabundle_fmt = attestation_doc
        .cabundle
        .iter()
        .map(|item| hex::encode(item.clone().into_vec()))
        .collect::<Vec<String>>()
        .join(", ");

    let output = match view {
        "bin" | "hex" | "bin_hex" | "cose_doc" | "cose_doc_bin" | "cose_doc_hex"
        | "cose_doc_bin_hex" => hex::encode(att_doc),

        "att_doc" | "att_doc_bin" | "att_doc_hex" | "att_doc_bin_hex" => {
            hex::encode(attestation_doc_bytes.clone())
        }

        "att_doc_user_data" | "att_doc_user_data_json" | "att_doc_user_data_json_hex" => {
            let json_value = serde_json::json!({
                "user_data": hex::encode(att_doc_user_data_bytes.clone()),
                "public_key": hex::encode(attestation_doc.public_key.clone().unwrap_or(ByteBuf::new()).into_vec()),
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                format!("Error formatting to JSON: {}", e)
            })
        }

        "pcr" | "pcrs" => {
            let json_value = serde_json::json!({
                "PCRs": pcrs_fmt
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                format!("Error formatting to JSON: {}", e)
            })
        }

        "json_hex" => {
            let json_value = serde_json::json!({
                "protected_header": header_protected_str,
                "unprotected_header": header_unprotected_str,
                "payload": {
                    "module_id": attestation_doc.module_id,
                    "digest": LocalNsmDigest(attestation_doc.digest).to_string(),
                    "timestamp": attestation_doc.timestamp.to_string(),
                    "PCRs": pcrs_fmt,
                    "certificate": hex::encode(attestation_doc.certificate.clone().into_vec()),
                    "ca_bundle": cabundle_fmt,
                    "public_key": hex::encode(attestation_doc.public_key.clone().unwrap_or(ByteBuf::new()).into_vec()),
                    "user_data": att_doc_user_data_json_string,
                    "nonce": hex::encode(attestation_doc.nonce.clone().unwrap_or(ByteBuf::new()).into_vec()),
                },
                "signature": hex::encode(attestation_doc_signature.clone()),
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                format!("Error formatting to JSON: {}", e)
            })
        }

        "json_str" => {
            let json_value = serde_json::json!({
                "protected_header": header_protected_str,
                "unprotected_header": header_unprotected_str,
                "payload": attestation_doc_json_string,
                "signature": hex::encode(attestation_doc_signature.clone()),
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                format!("Error formatting to JSON: {}", e)
            })
        }

        "json_debug" => {
            let json_value = serde_json::json!({
                "protected_header": format!("{:?}", protected_header),
                "unprotected_header": format!("{:?}", unprotected_header),
                "payload": attestation_doc_json_string,
                "signature": format!("{:?}", attestation_doc_signature.clone()),
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                format!("Error formatting to JSON: {}", e)
            })
        }

        "debug" => format!("{:#?}", cose_doc),

        "debug_pretty_print" => {
            let json_value = serde_json::json!({
                "protected_header": format!("{:?}", protected_header),
                "unprotected_header": format!("{:?}", unprotected_header),
                "payload": {
                    "module_id": attestation_doc.module_id,
                    "digest": LocalNsmDigest(attestation_doc.digest).to_string(),
                    "timestamp": attestation_doc.timestamp.to_string(),
                    "PCRs": format!("{:?}", attestation_doc.pcrs),
                    "certificate": format!("{:?}", attestation_doc.certificate),
                    "ca_bundle": format!("{:?}", attestation_doc.cabundle),
                    "public_key": format!("{:?}", attestation_doc.public_key),
                    "user_data": {
                        "file_path": att_doc_user_data.file_path,
                        "sha3_hash": att_doc_user_data.sha3_hash,
                        "vrf_proof": att_doc_user_data.vrf_proof,
                        "vrf_cipher_suite": att_doc_user_data.vrf_cipher_suite.to_string(),
                    },
                    "nonce": format!("{:?}", attestation_doc.nonce),
                },
                "signature": format!("{:?}", attestation_doc_signature),
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                format!("Error formatting to JSON: {}", e)
            })
        }

        _ => {
            let json_value = serde_json::json!({
                "att_doc_hex": hex::encode(att_doc),
                "supported_views": [
                    "bin_hex", "att_doc", "att_doc_user_data", "pcrs",
                    "json_hex", "json_str", "json_debug", "debug", "debug_pretty_print"
                ]
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(|e| {
                error!("Error formatting to JSON: {}", e);
                format!("Error formatting to JSON: {}", e)
            })
        }
    };

    Ok(output)
}
