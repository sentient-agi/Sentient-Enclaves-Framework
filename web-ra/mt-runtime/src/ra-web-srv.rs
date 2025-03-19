/// Remote attestation web-server for Sentient Enclaves Framework

use axum::{
    extract::{Query, State},
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Html},
    routing::{get, post},
    Router,
    BoxError,
    Json,
};
use axum_extra::extract::Host;
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use sha3::{Digest, Sha3_512};

use std::option::Option;
use std::{
    collections::{HashMap, BTreeMap, BTreeSet},
    sync::Arc,
    pin::Pin,
    future::Future,
    io::{self, Read},
    fs::{self, DirEntry},
    path::{Path as StdPath, PathBuf},
    net::SocketAddr, time::Duration,
};
use std::net::IpAddr;

use async_std::prelude::FutureExt;
use tokio::sync::Mutex;
use tokio::signal;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use tracing_subscriber::fmt::format;
use tracing::{debug, info, error};

use aws_nitro_enclaves_nsm_api::api::{Digest as NsmDigest, Request, Response, AttestationDoc};
use aws_nitro_enclaves_nsm_api::driver::{nsm_exit, nsm_init, nsm_process_request};
use serde_bytes::ByteBuf;
use aws_nitro_enclaves_cose::CoseSign1;
use aws_nitro_enclaves_cose::crypto::openssl::Openssl;
use futures::AsyncReadExt;
use futures::stream::Count;
use rand_core::{RngCore, OsRng}; // requires 'getrandom' feature

use parking_lot::RwLock;

use openssl::pkey::{PKey, Private, Public};

use vrf::openssl::{CipherSuite, Error, ECVRF};
use vrf::VRF;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct Ports {
    http: u16,
    https: u16,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct Keys {
    sk4proofs: Option<String>,
    sk4docs: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct Config {
    nsm_fd: Option<String>,
    ports: Ports,
    keys: Keys,
}

#[derive(Default, Debug, Clone)]
struct AppConfig {
    inner: Arc<RwLock<Config>>,
}

#[derive(Default, Debug, Clone)]
struct AppState {
    nsm_fd: i32,
    sk4proofs: Vec<u8>,
    sk4docs: Vec<u8>,
}

#[derive(Default, Debug, Clone)]
struct AppCache {
    att_data: HashMap<String, AttData>,
}

#[derive(Default, Debug, Clone)]
struct AttData {
    file_path: String,
    sha3_hash: String,
    vrf_proof: String,
    att_doc: Vec<u8>,
}

#[derive(Default, Debug, Clone)]
struct ServerState {
    tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<io::Result<Vec<u8>>>>>>,
    results: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    app_state: Arc<RwLock<AppState>>,
    app_cache: Arc<RwLock<AppCache>>,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct GenerateRequest {
    path: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config_path = "./.config/config.toml";
    let raw_config_string = std::fs::read_to_string(config_path)
        .expect("Missing `config.toml` file.");
    let config = toml::from_str::<Config>(raw_config_string.as_str()).expect("Failed to parse `config.toml` file.");
    let app_config = AppConfig {
        inner: Arc::new(RwLock::new(config))
    };

    let ports = Ports {
        http: app_config.inner.read().ports.http,
        https: app_config.inner.read().ports.https,
    };

    let fd = if let Some(fd) = app_config.inner.read().clone().nsm_fd {
        match fd.as_str() {
            // file descriptor returned by NSM device initialization function
            "nsm" | "nsm_dev" => nsm_init(),
            // testing file descriptor, for usage with NSM device emulator
            "debug" => 3,
            // particular file descriptor, for usage with NSM device emulator
            nsm_fd => nsm_fd.parse::<i32>().unwrap(),
        }
    } else { 3 }; // testing file descriptor, for usage with NSM device emulator
    assert!(fd >= 0, "[Error] NSM initialization returned {}.", fd);
    info!("NSM device initialized.");

    let state = Arc::new(ServerState {
        tasks: Arc::new(Mutex::new(HashMap::new())),
        results: Arc::new(Mutex::new(HashMap::new())),
        app_state: Arc::new(RwLock::new(AppState::default())),
        app_cache: Arc::new(RwLock::new(AppCache::default())),
    });

    // Share NSM file descriptor for future calls to NSM device
    state.app_state.write().nsm_fd = fd;

    let skey_opt = {
        let lock = app_config.inner.read();
        let val = lock.keys.sk4proofs.clone();
        val
    }; // lock dropped here
    if let Some(sk4proofs) = skey_opt {
        match sk4proofs.as_str() {
            "" => {
                let (skey, _pkey) = generate_ec256_keypair();
                let skey_bytes = skey.private_key_to_pem_pkcs8().unwrap();
                info!("SK for VRF Proofs length: {:?}; {:?}", skey_bytes.len(), skey_bytes.clone());

                state.app_state.write().sk4proofs = skey_bytes.clone();
                let skey_string = String::from_utf8(skey_bytes.clone()).unwrap();
                std::fs::create_dir_all("./.keys/").unwrap();
                std::fs::write("./.keys/sk4proofs.pkcs8.pem", skey_string).unwrap();

                let skey_hex = hex::encode(skey_bytes);

                let mut config = app_config.inner.write();
                info!("App Config locked: {:?};", config);
                config.keys.sk4proofs = Some(skey_hex.clone());
                info!("App Config: {:?}; {:?}", config, skey_hex.clone());
                drop(config);

                let app_config_clone = app_config.inner.read().to_owned();
                let toml_config = toml::to_string(&app_config_clone).unwrap();
                std::fs::create_dir_all("./.config/").unwrap();
                std::fs::write(&config_path, &toml_config).unwrap();
            },
            _ => {
                // Check if SK for proof generation has correct length
                if hex::decode(sk4proofs.clone()).unwrap().len() != 237 {
                    panic!("[Error] SK length for VRF Proofs mismatch.");
                };
                state.app_state.write().sk4proofs = hex::decode(sk4proofs).unwrap();
                let state = state.app_state.read().clone();
                let config = app_config.inner.read().clone();
                info!("App State & Config:\n {:?}\n {:?}", state, config);
            },
        }
    } else {
        let (skey, _pkey) = generate_ec256_keypair();
        let skey_bytes = skey.private_key_to_pem_pkcs8().unwrap();
        info!("SK for VRF Proofs length: {:?}; {:?}", skey_bytes.len(), skey_bytes.clone());

        state.app_state.write().sk4proofs = skey_bytes.clone();
        let skey_string = String::from_utf8(skey_bytes.clone()).unwrap();
        std::fs::create_dir_all("./.keys/").unwrap();
        std::fs::write("./.keys/sk4proofs.pkcs8.pem", skey_string).unwrap();

        let skey_hex = hex::encode(skey_bytes);

        let mut config = app_config.inner.write();
        info!("App Config locked: {:?};", config);
        config.keys.sk4proofs = Some(skey_hex.clone());
        info!("App Config: {:?}; {:?}", config, skey_hex.clone());
        drop(config);

        let app_config_clone = app_config.inner.read().to_owned();
        let toml_config = toml::to_string(&app_config_clone).unwrap();
        std::fs::create_dir_all("./.config/").unwrap();
        std::fs::write(&config_path, &toml_config).unwrap();
    };

    let skey_opt = {
        let lock = app_config.inner.read();
        let val = lock.keys.sk4docs.clone();
        val
    }; // lock dropped here
    if let Some(sk4docs) = skey_opt {
        match sk4docs.as_str() {
            "" => {
                let (skey, _pkey) = generate_ec512_keypair();
                let skey_bytes = skey.private_key_to_pem_pkcs8().unwrap();
                info!("SK for attestation documents signing length: {:?}; {:?}", skey_bytes.len(), skey_bytes.clone());

                state.app_state.write().sk4docs = skey_bytes.clone();
                let skey_string = String::from_utf8(skey_bytes.clone()).unwrap();
                std::fs::create_dir_all("./.keys/").unwrap();
                std::fs::write("./.keys/sk4docs.pkcs8.pem", skey_string).unwrap();

                let skey_hex = hex::encode(skey_bytes);

                let mut config = app_config.inner.write();
                info!("App Config locked: {:?};", config);
                config.keys.sk4docs = Some(skey_hex.clone());
                info!("App Config: {:?}; {:?}", config, skey_hex.clone());
                drop(config);

                let app_config_clone = app_config.inner.read().to_owned();
                let toml_config = toml::to_string(&app_config_clone).unwrap();
                std::fs::create_dir_all("./.config/").unwrap();
                std::fs::write(&config_path, &toml_config).unwrap();
            },
            _ => {
                // Check if SK for attestation documents signing has correct length
                if hex::decode(sk4docs.clone()).unwrap().len() != 384 {
                    panic!("[Error] SK length for attestation documents signing mismatch.");
                };
                state.app_state.write().sk4docs = hex::decode(sk4docs).unwrap();
                let state = state.app_state.read().clone();
                let config = app_config.inner.read().clone();
                info!("App State & Config:\n {:?}\n {:?}", state, config);
            },
        }
    }  else {
        let (skey, _pkey) = generate_ec512_keypair();
        let skey_bytes = skey.private_key_to_pem_pkcs8().unwrap();
        info!("SK for attestation documents signing length: {:?}; {:?}", skey_bytes.len(), skey_bytes.clone());

        state.app_state.write().sk4docs = skey_bytes.clone();
        let skey_string = String::from_utf8(skey_bytes.clone()).unwrap();
        std::fs::create_dir_all("./.keys/").unwrap();
        std::fs::write("./.keys/sk4docs.pkcs8.pem", skey_string).unwrap();

        let skey_hex = hex::encode(skey_bytes);

        let mut config = app_config.inner.write();
        info!("App Config locked: {:?};", config);
        config.keys.sk4docs = Some(skey_hex.clone());
        info!("App Config: {:?}; {:?}", config, skey_hex.clone());
        drop(config);

        let app_config_clone = app_config.inner.read().to_owned();
        let toml_config = toml::to_string(&app_config_clone).unwrap();
        std::fs::create_dir_all("./.config/").unwrap();
        std::fs::write(&config_path, &toml_config).unwrap();
    };

    //Create a handle for our TLS server so the shutdown signal can all shutdown
    let handle = axum_server::Handle::new();
    //save the future for easy shutting down of redirect server
    let shutdown_future = shutdown_signal(handle.clone(), Arc::clone(&state.app_state));
    // spawn a second server to redirect http requests to this server
    tokio::spawn(redirect_http_to_https(ports.clone(), shutdown_future));

    // configure certificate and private key used by https
    let cert_dir = std::env::var("CERT_DIR")
        .unwrap_or_else(|e| {
            error!("CERT_DIR env var is empty or not set: {:?}", e);
            "/app/certs/".to_string()
        });
    let tls_config = RustlsConfig::from_pem_file(
        PathBuf::from(&cert_dir)
            .join("cert.pem"),
        PathBuf::from(&cert_dir)
            .join("key.pem"),
    )
    .await
    .unwrap();

    let app = Router::new()
        .route("/generate", post(generate_handler))
        .route("/ready/", get(ready_handler))
//        .route("/hash/", get(hash_handler))
        .route("/hashes/", get(hashes))
        .route("/hash/", get(hashes))
//        .route("/proofs/", get(proofs))
//        .route("/proof/", get(proofs))
//        .route("/docs/", get(docs))
//        .route("/doc/", get(docs))
//        .route("/pubkeys/", get(pubkeys))
//        .route("/verify_proof/", get(verify_proof))
//        .route("/verify_doc/", get(verify_doc))
        .route("/echo/", get(echo))
        .route("/hello/", get(hello))
        .route("/nsm_desc", get(nsm_desc).with_state(Arc::clone(&state.app_state)))
        .route("/rng_seq", get(rng_seq).with_state(Arc::clone(&state.app_state)))
//        .route("/gen_att_doc/", get(gen_att_doc))
        .with_state(state.clone());

    // run https server
    use std::str::FromStr;
    let listening_address = core::net::SocketAddr::new(
        IpAddr::V4(
            core::net::Ipv4Addr::from_str("127.0.0.1").unwrap()
        ),
        ports.https
    );
    debug!("listening on {listening_address}");
    axum_server::bind_rustls(listening_address, tls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn generate_handler(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<GenerateRequest>,
) -> impl IntoResponse {
    let path_str = payload.path.clone();
    let path = StdPath::new(&path_str);

    // Check if the path exists
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                format!("Path not found: {}", e),
            );
        }
    };

    let is_dir = metadata.is_dir();

    let state_clone = state.clone();

    // Spawn the processing task
    tokio::spawn(async move {
        let path_buf = StdPath::new(&path_str).to_path_buf();
        if let Err(e) = visit_files_recursively(&path_buf, state_clone).await {
            eprintln!("Error processing path {}: {}", path_buf.display(), e);
        }
    });

    let message = if is_dir {
        "Started processing directory"
    } else {
        "Started processing file"
    };
    (StatusCode::ACCEPTED, message.to_string())
}

fn visit_files_recursively<'a>(
    path: &'a StdPath,
    state: Arc<ServerState>
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

            // Track the task and handle its completion
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

                            // Proofs gen logic

                            let app_state = app_state_clone.read().clone();

                            let skey4proofs_bytes = app_state.sk4proofs;
                            let skey4proofs_pkey = PKey::private_key_from_pem(skey4proofs_bytes.as_slice()).unwrap();
                            let skey4proofs_eckey = skey4proofs_pkey.ec_key().unwrap();
                            let skey4proofs_bignum = skey4proofs_eckey.private_key().to_owned().unwrap();
                            let skey4proofs_vec = skey4proofs_bignum.to_vec();
                            let vrf_proof = vrf_proof(skey4proofs_vec.as_slice(), hash.as_slice()).unwrap();

                            // Docs gen logic

                            let mut app_cache = app_cache_clone.write();

                            let fd = app_state.nsm_fd;
                            let nonce = get_randomness_sequence(fd.clone(), 1024);
                            let alg = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::SECP256K1).unwrap();
                            let skey4proofs_ec_pubkey = openssl::ec::EcKey::from_public_key(&alg, skey4proofs_eckey.public_key()).unwrap();
                            let skey4proofs_pkey_pubkey = PKey::from_ec_key(skey4proofs_ec_pubkey).unwrap();
                            let skey4proofs_pubkey_pem = skey4proofs_pkey_pubkey.public_key_to_pem().unwrap();

                            let att_doc = get_attestation_doc(
                                fd,
                                Some(ByteBuf::from(vrf_proof.clone())),
                                Some(ByteBuf::from(nonce.clone())),
                                Some(ByteBuf::from(skey4proofs_pubkey_pem.clone())),
                            );

                            app_cache.att_data.insert(file_path_clone.clone(), AttData {
                                file_path: file_path_clone.clone(),
                                sha3_hash: hex::encode(hash.clone()),
                                vrf_proof: hex::encode(vrf_proof.clone()),
                                att_doc: att_doc.clone(),
                            });
                        }
                        Ok(Err(e)) => {
                            eprintln!("Error hashing file: {}", e);
                            error!("Error hashing file: {}", e);
                        }
                        Err(e) => {
                            eprintln!("Task panicked: {}", e);
                            error!("Task panicked: {}", e);
                        }
                    }

                    // Remove the task from HashMap after awaiting it (after it completes)
                    let mut tasks = tasks_clone.lock().await;
                    tasks.remove(&file_path_clone);
                }
            });

            // Yield to allow other async tasks to make progress
            tokio::task::yield_now().await;
        }
        Ok(())
    })
}

fn hash_file(file_path: &str) -> io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(file_path)?;
    let mut hasher = Sha3_512::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_vec())
}

fn vrf_proof(message: &[u8], secret_key: &[u8]) -> Result<Vec<u8>, String> {
    let mut vrf  = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
    let public_key = vrf.derive_public_key(&secret_key).unwrap();
    let proof = vrf.prove(&secret_key, &message).unwrap();
    Ok(proof)
}

fn vrf_verify(message: &[u8], proof: &[u8], public_key: &[u8]) -> Result<bool, Error> {
    let mut vrf  = ECVRF::from_suite(CipherSuite::SECP256K1_SHA256_TAI).unwrap();
    let hash = vrf.proof_to_hash(&proof).unwrap();
    let outcome = vrf.verify(&public_key, &proof, &message);
    match outcome {
        Ok(outcome) => {
            info!("VRF proof is valid!");
            let result = if hash == outcome { true } else { false };
            Ok(result)
        }
        Err(e) => {
            error!("VRF proof is not valid! Error: {}", e);
            Err(e)
        }
    }
}

async fn ready_handler(
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    let file_path = query_params.get("path").unwrap().to_owned();
    let results = state.results.lock().await;
    if results.contains_key(&file_path) {
        (StatusCode::OK, "Ready".to_string())
    } else {
        let tasks = state.tasks.lock().await;
        if tasks.contains_key(&file_path) {
            (StatusCode::PROCESSING, "Processing".to_string())
        } else {
            (StatusCode::NOT_FOUND, "Not found".to_string())
        }
    }
}

async fn hash_handler(
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    let file_path = query_params.get("path").unwrap().to_owned();
    let results = state.results.lock().await;
    match results.get(&file_path) {
        Some(hash) => (StatusCode::OK, hex::encode(hash)),
        None => {
            let tasks = state.tasks.lock().await;
            if tasks.contains_key(&file_path) {
                (StatusCode::ACCEPTED, "Processing".to_string())
            } else {
                (StatusCode::NOT_FOUND, "Not found".to_string())
            }
        }
    }
}

/// Testing echo endpoint handler for API protocol and parameters parsing various testing purposes
async fn echo(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let fd = server_state.app_state.read().nsm_fd;
    info!("fd: {fd:?}");

    let file_path = query_params.get("path").unwrap().to_owned();
    info!("File path: {:?}", file_path);

    let response = query_params.iter()
        .map(
            |(key, val)| {
                let output = format!("Query Parameter: {:?}; Value: {:?};\n", key, val);
                info!("{output:?}");
                output
            }
        )
        .collect::<Vec<String>>()
        .join("\n");
    info!("{response:?}");

    (StatusCode::OK, response)
}

/// A handler stub for testing purposes
async fn hello(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
        info!("{query_params:?}");

        let fd = server_state.app_state.read().nsm_fd;
        info!("fd: {fd:?}");

        let path = query_params.get("path").unwrap().to_owned();
        info!("Path: {:?}", path);

        match query_params.get("view").unwrap().as_str() {
            "bin" | "raw" => (),
            "hex" => (),
            "fmt" | "str" => (),
            "json" => (),
            _ => (),
        }

        (StatusCode::OK, Html("<h1>Hello, World!</h1>\n"))
    }

async fn hashes(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
        info!("{query_params:?}");

        let fd = server_state.app_state.read().nsm_fd;
        info!("fd: {fd:?}");

        let path = query_params.get("path").unwrap().to_owned();
        info!("Path: {:?}", path);

        let hashes = server_state.results.lock().await;
        let response = hashes.iter()
            .filter(
                |(key, _)|
                    key.contains(path.as_str())
            ).map(
                |(path, hash)| {
                    let output = format!("Path: {:?}; Hash: {:?};", path, hex::encode(hash.as_slice()));
                    info!("{output:?}");
                    output
                }
            )
            .collect::<Vec<String>>()
            .join("\n");
        info!("{response:?}");

        (StatusCode::OK, response)
    }

async fn nsm_desc(
    Query(query_params): Query<HashMap<String, String>>,
    State(app_state): State<Arc<RwLock<AppState>>>
) -> String {
    info!("{query_params:?}");
    let fd = app_state.read().nsm_fd;
    let description = get_nsm_description(fd).unwrap();
    assert_eq!(
        description.max_pcrs, 32,
        "[Error] NSM PCR count is {}.",
        description.max_pcrs
    );
    assert!(
        !description.module_id.is_empty(),
        "[Error] NSM module ID is missing."
    );

    info!(
        "NSM description: [major: {}, minor: {}, patch: {}, module_id: {}, max_pcrs: {}, locked_pcrs: {:?}, digest: {:?}].",
        description.version_major,
        description.version_minor,
        description.version_patch,
        description.module_id,
        description.max_pcrs,
        description.locked_pcrs,
        description.digest
    );

    format!(
        "NSM description: [major: {}, minor: {}, patch: {}, module_id: {}, max_pcrs: {}, locked_pcrs: {:?}, digest: {:?}].\n",
        description.version_major,
        description.version_minor,
        description.version_patch,
        description.module_id,
        description.max_pcrs,
        description.locked_pcrs,
        description.digest
    )
}

async fn rng_seq(
    Query(query_params): Query<HashMap<String, String>>,
    State(app_state): State<Arc<RwLock<AppState>>>
) -> String {
    info!("{query_params:?}");
    let fd = app_state.read().nsm_fd;
    let randomness_sequence = get_randomness_sequence(fd, 2048);
    format!("{:?}\n", hex::encode(randomness_sequence))
}

async fn gen_att_doc(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> String {
    info!("{query_params:?}");
    let fd = server_state.app_state.read().nsm_fd;

    let mut random_user_data = [0u8; 1024];
    OsRng.fill_bytes(&mut random_user_data);
    let mut random_nonce = [0u8; 1024];
    OsRng.fill_bytes(&mut random_nonce);
    let mut random_public_key = [0u8; 1024];
    OsRng.fill_bytes(&mut random_public_key);

    let document = get_attestation_doc(
        fd,
        Some(ByteBuf::from(&random_user_data[..])),
        Some(ByteBuf::from(&random_nonce[..])),
        Some(ByteBuf::from(&random_public_key[..])),
    );

    let cose_doc = CoseSign1::from_bytes(document.as_slice()).unwrap();
    let (protected_header, attestation_doc_bytes) =
        cose_doc.get_protected_and_payload::<Openssl>(None).unwrap();
    println!("Protected header: {:?}", protected_header);
    let unprotected_header = cose_doc.get_unprotected();
    println!("Unprotected header: {:?}", unprotected_header);
    let attestation_doc_signature = cose_doc.get_signature();
    let attestation_doc = AttestationDoc::from_binary(&attestation_doc_bytes[..]).unwrap();
    println!("Attestation document: {:?}", attestation_doc);
    println!("Attestation document signature: {:?}", hex::encode(attestation_doc_signature.clone()));

    format!("Attestation document: {:?}\n\
             Attestation document signature: {:?}\n",
            attestation_doc,
            hex::encode(attestation_doc_signature),
    )
}

/// Randomly generate PRIME256V1/P-256 key to use for validating signing internally
fn generate_ec256_keypair() -> (PKey<Private>, PKey<Public>) {
//    let alg = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::X9_62_PRIME256V1).unwrap();
    let alg = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::SECP256K1).unwrap();
    let ec_private = openssl::ec::EcKey::generate(&alg).unwrap();
    let ec_public =
        openssl::ec::EcKey::from_public_key(&alg, ec_private.public_key()).unwrap();
    (
        PKey::from_ec_key(ec_private).unwrap(),
        PKey::from_ec_key(ec_public).unwrap(),
    )
}

/// Randomly generate SECP521R1/P-512 key to use for validating signing internally
fn generate_ec512_keypair() -> (PKey<Private>, PKey<Public>) {
//    let alg = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::SECP521R1).unwrap();
    let alg = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::SECP521R1).unwrap();
    let ec_private = openssl::ec::EcKey::generate(&alg).unwrap();
    let ec_public =
        openssl::ec::EcKey::from_public_key(&alg, ec_private.public_key()).unwrap();
    (
        PKey::from_ec_key(ec_private).unwrap(),
        PKey::from_ec_key(ec_public).unwrap(),
    )
}

struct NsmDescription {
    version_major: u16,
    version_minor: u16,
    version_patch: u16,
    module_id: String,
    max_pcrs: u16,
    locked_pcrs: BTreeSet<u16>,
    digest: NsmDigest,
}

fn get_nsm_description(fd: i32) -> Result<NsmDescription, ()> {
    let response = nsm_process_request(fd, Request::DescribeNSM);
    match response {
        Response::DescribeNSM {
            version_major,
            version_minor,
            version_patch,
            module_id,
            max_pcrs,
            locked_pcrs,
            digest,
        } => Ok(NsmDescription {
            version_major,
            version_minor,
            version_patch,
            module_id,
            max_pcrs,
            locked_pcrs,
            digest,
        }),
        _ => {
            error!(
                "[Error] Request::DescribeNSM got invalid response: {:?}",
                response
            );
            eprintln!("[Error] Request::DescribeNSM got invalid response: {:?}", response);
            Err(())
        }
    }
}

fn get_randomness_sequence(fd: i32, count_bytes: u32) -> Vec<u8> {
    let mut prev_random: Vec<u8> = vec![];
    let mut random: Vec<u8> = vec![];

    for _ in 0..count_bytes {
        random = match nsm_process_request(fd, Request::GetRandom) {
            Response::GetRandom { random } => {
                assert!(!random.is_empty());
                assert!(prev_random != random);
                prev_random = random.clone();
                info!("Random bytes: {:?}", random.clone());
                random
            }
            resp => {
                error!(
                    "GetRandom: expecting Response::GetRandom, but got {:?} instead",
                    resp
                );
                vec![]
            },
        }
    };
    random
}

fn get_attestation_doc (
    fd: i32,
    user_data: Option<ByteBuf>,
    nonce: Option<ByteBuf>,
    public_key: Option<ByteBuf>,
) -> Vec<u8> {
    let response = nsm_process_request(
        fd,
        Request::Attestation {
            user_data,
            nonce,
            public_key,
        },
    );
    match response {
        Response::Attestation { document } => {
            assert_ne!(document.len(), 0, "[Error] COSE document is empty.");
            info!("COSE document length: {:?} bytes", document.len());
            // println!("Attestation document: {:?}", document);
            document
        }
        _ => {
            error!(
                "[Error] Request::Attestation got invalid response: {:?}",
                response
            );
            vec![]
        },
    }
}

async fn redirect_http_to_https<F>(ports: Ports, signal: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let ports_clone = ports.clone();
    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports_clone) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], ports.http));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    debug!("listening on {addr}");
    axum::serve(listener, redirect.into_make_service())
        .with_graceful_shutdown(signal)
        .await
        .unwrap();
}

async fn shutdown_signal(handle: axum_server::Handle, app_state: Arc<RwLock<AppState>>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Received termination signal shutting down");
    let fd = app_state.read().nsm_fd;
    // close device file descriptor before app exit
    nsm_exit(fd);
    info!("NSM device closed.");
    handle.graceful_shutdown(Some(Duration::from_secs(10))); // 10 secs is how long docker will wait to force shutdown
}
