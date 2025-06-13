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
use serde_json::{json, Value as JsonValue, to_string, to_string_pretty};
use serde_cbor::Value as CborValue;
use serde_cbor::Error as CborError;

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
    net::{SocketAddr, IpAddr, Ipv4Addr}, time::Duration,
};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::signal;

use parking_lot::RwLock;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use tracing_subscriber::fmt::format;
use tracing::{debug, info, error};

use aws_nitro_enclaves_nsm_api::api::{Digest as NsmDigest, Request, Response, AttestationDoc};
use aws_nitro_enclaves_nsm_api::driver::{nsm_exit, nsm_init, nsm_process_request};
use serde_bytes::ByteBuf;
use aws_nitro_enclaves_cose::{CoseSign1, error::CoseError};
use aws_nitro_enclaves_cose::crypto::openssl::Openssl;

use rand_core::{RngCore, OsRng}; // requires 'getrandom' feature

use openssl::pkey::{PKey, Private, Public};
use openssl::nid::Nid as CipherID;
use openssl::bn::{BigNum, BigNumContext};
use openssl::x509::{X509, X509StoreContext};
use openssl::x509::store::X509StoreBuilder;
use openssl::stack::Stack;
use openssl::asn1::Asn1Time;

use std::time::{SystemTime, UNIX_EPOCH};

use vrf::openssl::{CipherSuite, Error, ECVRF};
use vrf::VRF;

use async_nats::jetstream::kv::{
    Config as KVConfig,
    Entry as KVEntry,
    Store as KVStore,
    Operation as KVOperation
};
use async_nats::jetstream::context::Context as JSContext;
use async_nats::client::Client as NATSClient;
use async_nats::jetstream::kv::Keys as KVKeys;
use bytes::Bytes;
use futures::StreamExt;

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
    vrf_cipher_suite: Option<CipherSuite>,
    nats: Option<NATSMQPersistency>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct NATSMQPersistency {
    nats_persistency_enabled: Option<i32>, // == Some(1) > Some(0)
    nats_url: String, // "nats://127.0.0.1:4222"
    hash_bucket_name: String, // "fs_hashes"
    att_docs_bucket_name: String, // "fs_att_docs"
    persistent_client_name: String, // "ra_web_srv"
}

#[derive(Default, Debug, Clone)]
struct AppConfig {
    inner: Arc<RwLock<Config>>,
}

impl AppConfig {
    fn new_from_file(config_path: &str) -> Self {
        let raw_config_string = fs::read_to_string(config_path)
            .expect(format!("Failed to read config file via provided path. Missing '{}' file.", config_path).as_str());
        let config = toml::from_str::<Config>(raw_config_string.as_str())
            .expect(format!("Invalid TOML format. Failed to parse '{}' file.", config_path).as_str());
        AppConfig {
            inner: Arc::new(RwLock::new(config))
        }
    }

    fn save_to_file(&self, path: &str) {
        let config = self.inner.read();
        let toml_str = toml::to_string(&*config).expect("Failed to serialize config.");
        fs::write(path, toml_str).expect("Failed to write config file.");
    }

    fn update_nsm_fd(&self, new_nsm_fd: i32) {
        let mut config = self.inner.write();
        config.nsm_fd = Some(new_nsm_fd.to_string());
    }

    fn update_keys(&self, new_keys: Keys) {
        let mut config = self.inner.write();
        config.keys = Keys {
            sk4proofs: new_keys.sk4proofs,
            sk4docs: new_keys.sk4docs,
        };
        drop(config);
    }

    fn update_ports(&self, new_ports: Ports) {
        let mut config = self.inner.write();
        config.ports = Ports {
            http: new_ports.http,
            https: new_ports.https,
        };
    }

    fn get_nsm_fd(&self) -> i32 {
        let nsm_fd = if let Some(fd) = self.inner.read().clone().nsm_fd {
            match fd.as_str() {
                // file descriptor returned by NSM device initialization function
                "" | "nsm" | "nsm_dev" => nsm_init(),
                // testing file descriptor, for usage with NSM device emulator
                "debug" => 3,
                // particular file descriptor, for usage with NSM device emulator
                nsm_fd => nsm_fd.parse::<i32>().unwrap(),
            }
        } else { nsm_init() }; // testing file descriptor, for usage with NSM device emulator
        nsm_fd
    }

    fn get_keys(&self) -> Keys {
        self.inner.read().keys.clone()
    }

    fn get_ports(&self) -> Ports {
        self.inner.read().ports.clone()
    }

    fn get_vrf_cipher_suite(&self) -> CipherSuite {
        let config = self.inner.read().clone();
        if let Some(vrf_cipher_suite) = config.vrf_cipher_suite {
            vrf_cipher_suite
        } else { panic!("[Error] 'vrf_cipher_suite' not present in configuration file.") }
    }
}

trait CipherMapper {
    fn to_nid(&self) -> CipherID;
    fn to_string(&self) -> String;
    fn from_string(suite_string: &str) -> Result<CipherSuite, String>;
}

impl CipherMapper for CipherSuite {
    /// Convert CipherSuite to openssl::nid::Nid
    fn to_nid(&self) -> CipherID {
        match *self {
            CipherSuite::SECP256K1_SHA256_TAI => CipherID::SECP256K1,
            CipherSuite::P256_SHA256_TAI => CipherID::X9_62_PRIME256V1,
            CipherSuite::K163_SHA256_TAI => CipherID::SECT163K1,

            CipherSuite::SECP256R1_SHA256_TAI => CipherID::X9_62_PRIME256V1,
            CipherSuite::SECP384R1_SHA384_TAI => CipherID::SECP384R1,
            CipherSuite::SECP521R1_SHA512_TAI => CipherID::SECP521R1,

            CipherSuite::ECDSA_SECP256R1_SHA256_TAI => CipherID::ECDSA_WITH_SHA256,
            CipherSuite::ECDSA_SECP384R1_SHA384_TAI => CipherID::ECDSA_WITH_SHA384,
            CipherSuite::ECDSA_SECP521R1_SHA512_TAI => CipherID::ECDSA_WITH_SHA512,

            CipherSuite::SECT163K1_SHA256_TAI => CipherID::SECT163K1,
            CipherSuite::SECT163R1_SHA256_TAI => CipherID::SECT163R1,
            CipherSuite::SECT163R2_SHA256_TAI => CipherID::SECT163R2,
            CipherSuite::SECT193R1_SHA256_TAI => CipherID::SECT193R1,
            CipherSuite::SECT193R2_SHA256_TAI => CipherID::SECT193R2,
            CipherSuite::SECT233K1_SHA256_TAI => CipherID::SECT233K1,
            CipherSuite::SECT233R1_SHA256_TAI => CipherID::SECT233R1,
            CipherSuite::SECT239K1_SHA256_TAI => CipherID::SECT239K1,
            CipherSuite::SECT283K1_SHA384_TAI => CipherID::SECT283K1,
            CipherSuite::SECT283R1_SHA384_TAI => CipherID::SECT283R1,
            CipherSuite::SECT409K1_SHA384_TAI => CipherID::SECT409K1,
            CipherSuite::SECT409R1_SHA384_TAI => CipherID::SECT409R1,
            CipherSuite::SECT571K1_SHA512_TAI => CipherID::SECT571K1,
            CipherSuite::SECT571R1_SHA512_TAI => CipherID::SECT571R1,

            CipherSuite::BRAINPOOL_P256R1_SHA256_TAI => CipherID::BRAINPOOL_P256R1,
            CipherSuite::BRAINPOOL_P320R1_SHA256_TAI => CipherID::BRAINPOOL_P320R1,
            CipherSuite::BRAINPOOL_P384R1_SHA384_TAI => CipherID::BRAINPOOL_P384R1,
            CipherSuite::BRAINPOOL_P512R1_SHA512_TAI => CipherID::BRAINPOOL_P512R1,
        }
    }

    /// Convert CipherSuite type to corresponding string
    fn to_string(&self) -> String {
        let suite_string = match self {
            CipherSuite::SECP256K1_SHA256_TAI => "SECP256K1_SHA256_TAI",
            CipherSuite::P256_SHA256_TAI => "P256_SHA256_TAI",
            CipherSuite::K163_SHA256_TAI => "K163_SHA256_TAI",

            CipherSuite::SECP256R1_SHA256_TAI => "SECP256R1_SHA256_TAI",
            CipherSuite::SECP384R1_SHA384_TAI => "SECP384R1_SHA384_TAI",
            CipherSuite::SECP521R1_SHA512_TAI => "SECP521R1_SHA512_TAI",

            CipherSuite::ECDSA_SECP256R1_SHA256_TAI => "ECDSA_SECP256R1_SHA256_TAI",
            CipherSuite::ECDSA_SECP384R1_SHA384_TAI => "ECDSA_SECP384R1_SHA384_TAI",
            CipherSuite::ECDSA_SECP521R1_SHA512_TAI => "ECDSA_SECP521R1_SHA512_TAI",

            CipherSuite::SECT163K1_SHA256_TAI => "SECT163K1_SHA256_TAI",
            CipherSuite::SECT163R1_SHA256_TAI => "SECT163R1_SHA256_TAI",
            CipherSuite::SECT163R2_SHA256_TAI => "SECT163R2_SHA256_TAI",
            CipherSuite::SECT193R1_SHA256_TAI => "SECT193R1_SHA256_TAI",
            CipherSuite::SECT193R2_SHA256_TAI => "SECT193R2_SHA256_TAI",
            CipherSuite::SECT233K1_SHA256_TAI => "SECT233K1_SHA256_TAI",
            CipherSuite::SECT233R1_SHA256_TAI => "SECT233R1_SHA256_TAI",
            CipherSuite::SECT239K1_SHA256_TAI => "SECT239K1_SHA256_TAI",
            CipherSuite::SECT283K1_SHA384_TAI => "SECT283K1_SHA384_TAI",
            CipherSuite::SECT283R1_SHA384_TAI => "SECT283R1_SHA384_TAI",
            CipherSuite::SECT409K1_SHA384_TAI => "SECT409K1_SHA384_TAI",
            CipherSuite::SECT409R1_SHA384_TAI => "SECT409R1_SHA384_TAI",
            CipherSuite::SECT571K1_SHA512_TAI => "SECT571K1_SHA512_TAI",
            CipherSuite::SECT571R1_SHA512_TAI => "SECT571R1_SHA512_TAI",

            CipherSuite::BRAINPOOL_P256R1_SHA256_TAI => "BRAINPOOL_P256R1_SHA256_TAI",
            CipherSuite::BRAINPOOL_P320R1_SHA256_TAI => "BRAINPOOL_P320R1_SHA256_TAI",
            CipherSuite::BRAINPOOL_P384R1_SHA384_TAI => "BRAINPOOL_P384R1_SHA384_TAI",
            CipherSuite::BRAINPOOL_P512R1_SHA512_TAI => "BRAINPOOL_P512R1_SHA512_TAI",
        };
        suite_string.to_string()
    }

    /// Convert string with cipher suite to the corresponding CipherSuite type
    fn from_string(suite_string: &str) -> Result<CipherSuite, String> {
        let cipher_suite = match suite_string {
            "SECP256K1_SHA256_TAI" => CipherSuite::SECP256K1_SHA256_TAI,
            "P256_SHA256_TAI" => CipherSuite::P256_SHA256_TAI,
            "K163_SHA256_TAI" => CipherSuite::K163_SHA256_TAI,

            "SECP256R1_SHA256_TAI" => CipherSuite::SECP256R1_SHA256_TAI,
            "SECP384R1_SHA384_TAI" => CipherSuite::SECP384R1_SHA384_TAI,
            "SECP521R1_SHA512_TAI" => CipherSuite::SECP521R1_SHA512_TAI,

            "ECDSA_SECP256R1_SHA256_TAI" => CipherSuite::ECDSA_SECP256R1_SHA256_TAI,
            "ECDSA_SECP384R1_SHA384_TAI" => CipherSuite::ECDSA_SECP384R1_SHA384_TAI,
            "ECDSA_SECP521R1_SHA512_TAI" => CipherSuite::ECDSA_SECP521R1_SHA512_TAI,

            "SECT163K1_SHA256_TAI" => CipherSuite::SECT163K1_SHA256_TAI,
            "SECT163R1_SHA256_TAI" => CipherSuite::SECT163R1_SHA256_TAI,
            "SECT163R2_SHA256_TAI" => CipherSuite::SECT163R2_SHA256_TAI,
            "SECT193R1_SHA256_TAI" => CipherSuite::SECT193R1_SHA256_TAI,
            "SECT193R2_SHA256_TAI" => CipherSuite::SECT193R2_SHA256_TAI,
            "SECT233K1_SHA256_TAI" => CipherSuite::SECT233K1_SHA256_TAI,
            "SECT233R1_SHA256_TAI" => CipherSuite::SECT233R1_SHA256_TAI,
            "SECT239K1_SHA256_TAI" => CipherSuite::SECT239K1_SHA256_TAI,
            "SECT283K1_SHA384_TAI" => CipherSuite::SECT283K1_SHA384_TAI,
            "SECT283R1_SHA384_TAI" => CipherSuite::SECT283R1_SHA384_TAI,
            "SECT409K1_SHA384_TAI" => CipherSuite::SECT409K1_SHA384_TAI,
            "SECT409R1_SHA384_TAI" => CipherSuite::SECT409R1_SHA384_TAI,
            "SECT571K1_SHA512_TAI" => CipherSuite::SECT571K1_SHA512_TAI,
            "SECT571R1_SHA512_TAI" => CipherSuite::SECT571R1_SHA512_TAI,

            "BRAINPOOL_P256R1_SHA256_TAI" => CipherSuite::BRAINPOOL_P256R1_SHA256_TAI,
            "BRAINPOOL_P320R1_SHA256_TAI" => CipherSuite::BRAINPOOL_P320R1_SHA256_TAI,
            "BRAINPOOL_P384R1_SHA384_TAI" => CipherSuite::BRAINPOOL_P384R1_SHA384_TAI,
            "BRAINPOOL_P512R1_SHA512_TAI" => CipherSuite::BRAINPOOL_P512R1_SHA512_TAI,

            _ => return Err("Wrong cipher suite string used.".to_string()),
        };
        Ok(cipher_suite)
    }
}

#[derive(Default, Debug, Clone)]
struct AppState {
    nsm_fd: i32,
    sk4proofs: Vec<u8>,
    sk4docs: Vec<u8>,
    vrf_cipher_suite: CipherSuite,
    nats_client: Option<NATSClient>,
    storage: Option<KVStore>,
}

#[derive(Default, Debug, Clone)]
struct AppCache {
    att_data: HashMap<String, AttData>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct AttData {
    file_path: String,
    sha3_hash: String,
    vrf_proof: String,
    vrf_cipher_suite: CipherSuite,
    att_doc: Vec<u8>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct AttUserData {
    file_path: String,
    sha3_hash: String,
    vrf_proof: String,
    vrf_cipher_suite: CipherSuite,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct AttProofData {
    file_path: String,
    sha3_hash: String,
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

#[derive(Default, Debug, Clone, Deserialize)]
struct VerifyPCRsRequest {
    pcrs: String,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct VerifyHashRequest {
    file_path: String,
    sha3_hash: String,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct VerifyProofRequest {
    user_data: String,
    public_key: String,
}

#[derive(Default, Debug, Clone, Deserialize)]
struct VerifyDocRequest {
    cose_doc_bytes: String,
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

    let default_config_path = format!("./.config/{}.config.toml", env!("CARGO_CRATE_NAME"));
    let config_path = default_config_path.as_str();
    let app_config = Arc::new(AppConfig::new_from_file(config_path));

    let ports = Ports {
        http: app_config.get_ports().http,
        https: app_config.get_ports().https,
    };

    let fd = app_config.get_nsm_fd();
    assert!(fd >= 0, "[Error] NSM initialization returned {}", fd);
    info!("NSM device initialized.");
    let vrf_cipher_suite = app_config.get_vrf_cipher_suite();

    let state = Arc::new(ServerState {
        tasks: Arc::new(Mutex::new(HashMap::new())),
        results: Arc::new(Mutex::new(HashMap::new())),
        app_state: Arc::new(RwLock::new(AppState::default())),
        app_cache: Arc::new(RwLock::new(AppCache::default())),
    });

    {
        let mut app_state = state.app_state.write();
        // Share NSM file descriptor for future calls to NSM device
        app_state.nsm_fd = fd;
        // Set VRF Cipher Suite
        app_state.vrf_cipher_suite = vrf_cipher_suite;
        drop(app_state);
    };

    let skey4proofs = {
        let config = app_config.inner.read();
        let sk4proofs = config.keys.sk4proofs.clone();
        let val = sk4proofs.unwrap_or_else(|| "".to_string());
        val
    }; // lock dropped here

    match skey4proofs.as_str() {
        "" => {
            let cipher = app_config.get_vrf_cipher_suite().to_nid();
            let (skey, _pkey) = generate_keypair(cipher);
            // let (skey, _pkey) = generate_ec256_keypair();
            let skey_bytes = skey.private_key_to_pem_pkcs8().unwrap();
            info!("SK for VRF Proofs length: {:?}; {:?}", skey_bytes.len(), skey_bytes.clone());

            state.app_state.write().sk4proofs = skey_bytes.clone();
            let skey_string = String::from_utf8(skey_bytes.clone()).unwrap();
            std::fs::create_dir_all("./.keys/").unwrap();
            std::fs::write("./.keys/sk4proofs.pkcs8.pem", skey_string).unwrap();

            let skey_hex = hex::encode(skey_bytes);

            info!("App Config: {:?};", app_config.inner.read().clone());
            app_config.update_keys(Keys {
                sk4proofs: Some(skey_hex.clone()),
                sk4docs: app_config.get_keys().sk4docs,
            });
            info!("App Config: {:?}; {:?}", app_config.inner.read().clone(), skey_hex.clone());

            std::fs::create_dir_all("./.config/").unwrap();
            app_config.save_to_file(config_path);
        },
        skey => {
            // Check if SK for proof generation has the correct length
            let skey_byte_len = hex::decode(skey).unwrap().len();
            match skey_byte_len {
                237 | 241 | 384 => (),
                _ => panic!("[Error] SK length for VRF Proofs mismatch."),
            };
            state.app_state.write().sk4proofs = hex::decode(skey).unwrap();
            let state = state.app_state.read().clone();
            let config = app_config.inner.read().clone();
            info!("App State & App Config:\n {:?}\n {:?}", state, config);
        },
    };

    let skey4docs = {
        let config = app_config.inner.read();
        let sk4docs = config.keys.sk4docs.clone();
        let val = sk4docs.unwrap_or_else(|| "".to_string());
        val
    }; // lock dropped here

    match skey4docs.as_str() {
        "" => {
            // let cipher = app_config.get_vrf_cipher_suite().to_nid();
            // let (skey, _pkey) = generate_keypair(cipher);
            let (skey, _pkey) = generate_ec512_keypair();
            let skey_bytes = skey.private_key_to_pem_pkcs8().unwrap();
            info!("SK for attestation documents signing length: {:?}; {:?}", skey_bytes.len(), skey_bytes.clone());

            state.app_state.write().sk4docs = skey_bytes.clone();
            let skey_string = String::from_utf8(skey_bytes.clone()).unwrap();
            std::fs::create_dir_all("./.keys/").unwrap();
            std::fs::write("./.keys/sk4docs.pkcs8.pem", skey_string).unwrap();

            let skey_hex = hex::encode(skey_bytes);

            info!("App Config: {:?};", app_config.inner.read().clone());
            app_config.update_keys(Keys {
                sk4proofs: app_config.get_keys().sk4proofs,
                sk4docs: Some(skey_hex.clone()),
            });
            info!("App Config: {:?}; {:?}", app_config.inner.read().clone(), skey_hex.clone());

            std::fs::create_dir_all("./.config/").unwrap();
            app_config.save_to_file(config_path);
        },
        skey => {
            // Check if SK for attestation documents signing has the correct length
            let skey_byte_len = hex::decode(skey).unwrap().len();
            match skey_byte_len {
                384 => (),
                _ => panic!("[Error] SK length for attestation documents signing mismatch."),
            };
            state.app_state.write().sk4docs = hex::decode(skey).unwrap();
            let state = state.app_state.read().clone();
            let config = app_config.inner.read().clone();
            info!("App State & App Config:\n {:?}\n {:?}", state, config);
        },
    };

    // Persistent layer and integration with enclave's service bus (based on NATS MQ and KV buckets)
    // Retrieve NATS configuration data
    let nats_config = app_config.inner.read().nats.clone().unwrap_or_else(|| NATSMQPersistency::default());
    // Non-blocking: delegate to orchestrator in its own thread the management of NATS KV buckets, walker, watcher and producer threads
    if nats_config.nats_persistency_enabled.is_some_and(|enabled| enabled > 0) {
        let app_state_clone = Arc::clone(&state.app_state);
        let app_cache_clone = Arc::clone(&state.app_cache);
        tokio::spawn(async move {
            if let Err(e) = nats_orchestrator(app_state_clone, app_cache_clone, nats_config).await {
                error!("[NATS Orchestrator] NATS Orchestrator error: {}", e);
            }
        });
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
            "/apps/certs/".to_string()
        });
    let tls_config = RustlsConfig::from_pem_file(
        PathBuf::from(&cert_dir)
            .join("cert.pem"),
        PathBuf::from(&cert_dir)
            .join("skey.pem"),
    )
    .await
    .unwrap();

    let app = Router::new()
        .route("/generate", post(generate_handler))
        .route("/readiness/", get(readiness))
        .route("/ready/", get(ready_handler))
        .route("/hashes/", get(hashes))
        .route("/hash/", get(hash_handler))
        .route("/proofs/", get(proofs))
        .route("/proof/", get(proof_handler))
        .route("/docs/", get(docs))
        .route("/doc/", get(doc_handler))
        .route("/pubkeys/", get(pubkeys).with_state(Arc::clone(&state.app_state)))
        .route("/verify_pcrs/", post(verify_pcrs).with_state(Arc::clone(&state.app_state)))
        .route("/verify_hash/", post(verify_hash).with_state(Arc::clone(&state.app_state)))
        .route("/verify_proof/", post(verify_proof).with_state(Arc::clone(&state.app_state)))
        .route("/verify_doc/", post(verify_doc).with_state(Arc::clone(&state.app_state)))
        .route("/verify_cert_valid/", post(verify_cert_valid).with_state(Arc::clone(&state.app_state)))
        .route("/verify_cert_bundle/", post(verify_cert_bundle).with_state(Arc::clone(&state.app_state)))
        .route("/echo/", get(echo))
        .route("/hello/", get(hello))
        .route("/nsm_desc", get(nsm_desc).with_state(Arc::clone(&state.app_state)))
        .route("/rng_seq", get(rng_seq).with_state(Arc::clone(&state.app_state)))
        .with_state(state.clone());

    // run https server
    use std::str::FromStr;
    let listening_address = SocketAddr::new(
        IpAddr::V4(
            Ipv4Addr::from_str("127.0.0.1").unwrap_or_else(
                |e| {
                    error!("{:?}", e);
                    Ipv4Addr::new(0, 0, 0, 0)
                }
            )
        ),
        ports.https
    );
    debug!("listening on {listening_address:?}");
    axum_server::bind_rustls(listening_address, tls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

/// NATS orchestrator task: sets up client, channels, spawns core logic tasks (walker, watcher, producer)
async fn nats_orchestrator(
    app_state: Arc<RwLock<AppState>>,
    app_cache: Arc<RwLock<AppCache>>,
    nats_config: NATSMQPersistency,
) -> Result<(), Box<dyn std::error::Error>> {

    // Retrieve NATS configuration data

    let nats_url = if nats_config.nats_url.is_empty() {
        "nats://127.0.0.1:4222".to_string()
    } else {
        nats_config.nats_url
    };

    let source_bucket = if nats_config.hash_bucket_name.is_empty() {
        "fs_hashes".to_string()
    } else {
        nats_config.hash_bucket_name
    };

    let target_bucket = if nats_config.att_docs_bucket_name.is_empty() {
        "fs_att_docs".to_string()
    } else {
        nats_config.att_docs_bucket_name
    };

    // Create NATS Client with NATS connection, connect to NATS
    let client = loop {
        match async_nats::connect(nats_url.as_str()).await {
            Ok(conn) => break conn,
            Err(e) => {
                error!("[NATS Orchestrator] Connection failed: {}, retrying...", e);
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    };
    info!("[NATS Orchestrator] Connected to NATS at {}", nats_url);

    // Check connection status
    info!("Connection status: {}", client.connection_state());
    client.flush().await?;
    info!("Connection verified!");

    // Save NATS Client in App State
    {
        let mut app_state = app_state.write();
        app_state.nats_client = Some(client.clone());
    };

    // Get JetStream context
    let js = async_nats::jetstream::new(client);

    // Source and target buckets
    let source_kv = get_or_create_kv(&js, source_bucket.as_str()).await?;
    let target_kv = get_or_create_kv(&js, target_bucket.as_str()).await?;

    // Save target NATS KV Store for attestation documents in App State
    {
        let mut app_state = app_state.write();
        app_state.storage = Some(target_kv.clone());
    };

    // Channels: walker -> producer, watcher -> producer
    let (walker_tx, walker_rx) = mpsc::channel::<(String, Bytes)>(1000);
    let (watcher_tx, watcher_rx) = mpsc::channel::<(String, Bytes)>(1000);

    // Spawn pipeline of logic components in separate threads
    tokio::spawn(walk_kv_entries(source_kv.clone(), walker_tx));
    tokio::spawn(watch_kv_changes(source_kv.clone(), watcher_tx));
    let app_state_clone = Arc::clone(&app_state);
    let app_cache_clone = Arc::clone(&app_cache);
    tokio::spawn(produce_kv_updates(walker_rx, watcher_rx, app_state_clone, app_cache_clone));

    Ok(())
}

/// KV bucket getter/creator
async fn get_or_create_kv(js: &JSContext, bucket_name: &str) -> Result<KVStore, Box<dyn std::error::Error>> {
    Ok(match js.get_key_value(bucket_name).await {
        Ok(kv) => kv,
        Err(_) => {
            info!("[NATS KV] Creating bucket '{}'", bucket_name);
            js.create_key_value(KVConfig {
                bucket: bucket_name.to_string(),
                history: 10,
                ..Default::default()
            })
            .await?
        }
    })
}

/// Walker task
async fn walk_kv_entries(store: KVStore, sender: mpsc::Sender<(String, Bytes)>) {
    info!("[NATS Walker] Walking through existing entries...");
    match store.keys().await {
        Ok(mut keys) => {
            while let Some(key_result) = keys.next().await {
                match key_result {
                    Ok(key) => match store.get(&key).await {
                        Ok(Some(bytes)) => {
                            let _ = sender.send((key.clone(), bytes)).await;
                        }
                        Ok(None) => {
                            error!("[NATS Walker] Key '{}' not found or value not set", key);
                        }
                        Err(e) => error!("[NATS Walker] Failed to read key '{}': {}", key, e),
                    },
                    Err(e) => error!("[NATS Walker] Failed to read keys: {}", e),
                }
            }
        }
        Err(e) => error!("[NATS Walker] Unable to list keys: {}", e),
    }
}

/// Watcher task
async fn watch_kv_changes(store: KVStore, sender: mpsc::Sender<(String, Bytes)>) {
    info!("[NATS Watcher] Watching for changes...");
    match store.watch_all().await {
        Ok(mut watcher) => {
            while let Some(entry_result) = watcher.next().await {
                match entry_result {
                    Ok(entry) => {
                        let key = entry.key.clone();
                        let val = entry.value;
                        let operation = match entry.operation {
                            KVOperation::Put => "PUT",
                            KVOperation::Delete => "DELETE",
                            KVOperation::Purge => "PURGE",
                        };
                        info!("[NATS Watcher] Operation: {} | Key: {} | Value: {:?}", operation, key, val);
                        let _ = sender.send((key, val)).await;
                    }
                    Err(e) => error!("[NATS Watcher] Watch error: {}", e),
                }
            }
        }
        Err(e) => error!("[NATS Watcher] Failed to start watcher: {}", e),
    }
}

/// Producer task
async fn produce_kv_updates(
    mut walker_rx: mpsc::Receiver<(String, Bytes)>,
    mut watcher_rx: mpsc::Receiver<(String, Bytes)>,
    app_state: Arc<RwLock<AppState>>,
    app_cache: Arc<RwLock<AppCache>>,
) {
    info!("[NATS Producer] Processing walker entries...");
    while let Some((key, val)) = walker_rx.recv().await {
        info!("[NATS Producer] Walker: Putting [{}] = {:?}", key, val);
        make_attestation_docs(key.as_str(), val.to_vec().as_slice(), Arc::clone(&app_state), Arc::clone(&app_cache));
    }

    info!("[NATS Producer] Watching updates from watcher...");
    while let Some((key, val)) = watcher_rx.recv().await {
        info!("[NATS Producer] Watcher: Putting [{}] = {:?}", key, val);
        make_attestation_docs(key.as_str(), val.to_vec().as_slice(), Arc::clone(&app_state), Arc::clone(&app_cache));
    }
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
                format!("Path not found: {:?}\n", e),
            )
        }
    };

    let is_dir = metadata.is_dir();

    let state_clone = state.clone();

    // Spawn the processing task
    tokio::spawn(async move {
        let path_buf = StdPath::new(&path_str).to_path_buf();
        if let Err(e) = visit_files_recursively(&path_buf, state_clone).await {
            eprintln!("Error processing path {:?}: {:?}", path_buf.display(), e);
            error!("Error processing path {:?}: {:?}", path_buf.display(), e);
        }
    });

    let message = if is_dir {
        "Started processing directory"
    } else {
        "Started processing file"
    };
    (StatusCode::ACCEPTED, format!("{:?}\n", message.to_string()))
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
                            make_attestation_docs(file_path_clone.clone().as_str(), hash.clone().as_slice(), app_state_clone, app_cache_clone);
                        }
                        Ok(Err(e)) => {
                            eprintln!("Error hashing file: {:?}", e);
                            error!("Error hashing file: {:?}", e);
                        }
                        Err(e) => {
                            eprintln!("Task panicked: {:?}", e);
                            error!("Task panicked: {:?}", e);
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

fn make_attestation_docs(
    file_path: &str,
    hash: &[u8],
    app_state: Arc<RwLock<AppState>>,
    app_cache: Arc<RwLock<AppCache>>,
) {
    let file_path_string = file_path.to_string();

    // Proofs gen logic

    let app_state = app_state.read().clone();

    let skey4proofs_bytes = app_state.sk4proofs;
    let skey4proofs_pkey = PKey::private_key_from_pem(skey4proofs_bytes.as_slice()).unwrap();
    let skey4proofs_eckey = skey4proofs_pkey.ec_key().unwrap();
    let skey4proofs_bignum = skey4proofs_eckey.private_key().to_owned().unwrap();
    let skey4proofs_vec = skey4proofs_bignum.to_vec();

    // VRF Proof hash based on private key generated for file path and file hash pair (emulation of CoW FS metadata for enclave's ramdisk FS)
    let att_proof_data = AttProofData{
        file_path: file_path_string.clone(),
        sha3_hash: hex::encode(hash),
    };
    let att_proof_data_json_bytes = serde_json::to_vec(&att_proof_data).unwrap();
    let cipher_suite = app_state.vrf_cipher_suite;
    let vrf_proof = vrf_proof(att_proof_data_json_bytes.as_slice(), skey4proofs_vec.as_slice(), cipher_suite.clone()).unwrap();

    // Docs gen logic

    let mut app_cache = app_cache.write();

    let fd = app_state.nsm_fd;
    let mut vrf  = ECVRF::from_suite(cipher_suite.clone()).unwrap();
    // Cryptographic nonce provided by the attestation document as a proof of authenticity of document and user data
    let nonce = vrf.generate_nonce(&skey4proofs_bignum, att_proof_data_json_bytes.as_slice()).unwrap().to_vec();
    // Random nonce provided by the attestation document as a proof of authenticity of document and user data
    // let nonce = get_randomness_sequence(fd.clone(), 512);
    let skey4proofs_pubkey = vrf.derive_public_key(skey4proofs_vec.as_slice()).unwrap();

    let att_user_data = AttUserData {
        file_path: file_path_string.clone(),
        sha3_hash: hex::encode(hash),
        vrf_proof: hex::encode(vrf_proof.clone()),
        vrf_cipher_suite: cipher_suite.clone(),
    };
    let att_user_data_json_bytes = serde_json::to_vec(&att_user_data).unwrap();

    let att_doc = get_attestation_doc(
        fd,
        Some(ByteBuf::from(att_user_data_json_bytes)),
        Some(ByteBuf::from(nonce.clone())),
        Some(ByteBuf::from(skey4proofs_pubkey.clone())),
    );

    let att_data = AttData {
        file_path: file_path_string.clone(),
        sha3_hash: hex::encode(hash),
        vrf_proof: hex::encode(vrf_proof.clone()),
        vrf_cipher_suite: cipher_suite.clone(),
        att_doc: att_doc.clone(),
    };

    app_cache.att_data.insert(file_path_string.clone(), att_data.clone());

    if let Some(store) = app_state.storage {
        let att_data_json_bytes = serde_json::to_vec(&att_data).unwrap();
        tokio::task::spawn(async move {
            let _ = store.put(&file_path_string, Bytes::from(att_data_json_bytes)).await.unwrap_or_else(
                |e| {
                    error!("[NATS Producer: attestation docs maker] Error while putting data to KV store: {}", e);
                    eprintln!("[NATS Producer: attestation docs maker] Error while putting data to KV store: {}", e);
                    0u64
                }
            );
        });
    };
}

/// Testing echo endpoint handler for API protocol and parameters parsing various testing purposes
async fn echo(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let fd = server_state.app_state.read().nsm_fd;
    info!("fd: {fd:?}");

    let file_path = query_params.get("path").unwrap_or(&String::from("./")).to_owned();
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

    (StatusCode::OK, format!("{:?}\n", response))
}

/// A handler stub for various testing purposes
async fn hello(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let fd = server_state.app_state.read().nsm_fd;
    info!("fd: {fd:?}");

    let path = query_params.get("path").unwrap_or(&String::from("./")).to_owned();
    info!("Path: {:?}", path);

    match query_params.get("view").unwrap_or(&String::from("hex")).as_str() {
        "bin" | "raw" => (),
        "hex" => (),
        "fmt" | "str" => (),
        "json" => (),
        _ => (),
    }

    (StatusCode::OK, Html("<h1>Hello, World!</h1>\n"))
}

async fn ready_handler(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let file_path = match query_params.get("path") {
        None => "".to_string(),
        Some(file_path) => { file_path.to_owned() },
    };
    if file_path.is_empty() {
        return (StatusCode::BAD_REQUEST, String::from("'Path' parameter is missing. Set the requested 'path' first.\n"))
    };
    info!("File path: {:?}", file_path);

    let results = server_state.results.lock().await;
    if results.contains_key(&file_path) {
        let json_value = json!({
            "file_path": file_path,
            "sha3_hash": hex::encode(results.get(&file_path).unwrap_or(&vec![])),
            "status": "Ready",
        });
        (StatusCode::OK, serde_json::to_string_pretty(&json_value).unwrap_or_else(
            |e| {
                error!("Error formatting to JSON: {:?}", e);
                format!("Error formatting to JSON: {:?}", e)
            }
        ))
    } else {
        let tasks = server_state.tasks.lock().await;
        if tasks.contains_key(&file_path) {
            let json_value = json!({
                "file_path": file_path,
                "status": "Processing",
            });
            (StatusCode::PROCESSING, serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            ))
        } else {
            let json_value = json!({
                "file_path": file_path,
                "status": "Not found",
            });
            (StatusCode::NOT_FOUND, serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            ))
        }
    }
}

async fn readiness(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let path = match query_params.get("path") {
        None => "".to_string(),
        Some(path) => { path.to_owned() },
    };
    if path.is_empty() {
        return (StatusCode::BAD_REQUEST, String::from("'Path' parameter is missing. Set the requested 'path' first.\n"))
    };
    info!("Path: {:?}", path);

    let results = server_state.results.lock().await;
    let mut ready_statuses = Vec::<String>::with_capacity(1000);
    for (file_path, hash) in results.iter() {
        if file_path.contains(path.as_str()) {
            let json_value = json!({
                "file_path": file_path,
                "sha3_hash": hex::encode(hash),
                "status": "Ready",
            });
            let status = serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            );
            debug!("{status:#?}");
            ready_statuses.push(status);
        }
    };
    if ready_statuses.is_empty() {
        let json_value = json!({
            "path": path,
            "status": "Not found",
        });
        let status = serde_json::to_string_pretty(&json_value).unwrap_or_else(
            |e| {
                error!("Error formatting to JSON: {:?}", e);
                format!("Error formatting to JSON: {:?}", e)
            }
        );
        debug!("{status:#?}");
        ready_statuses.push(status);
    };
    let ready_output = ready_statuses.join("\n");
    info!("{ready_output:#?}");

    let tasks = server_state.tasks.lock().await;
    let mut task_statuses = Vec::<String>::with_capacity(1000);
    for (file_path, _) in tasks.iter() {
        if file_path.contains(path.as_str()) {
            let json_value = json!({
                "file_path": file_path,
                "status": "Processing",
            });
            let status = serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            );
            debug!("{status:#?}");
            task_statuses.push(status);
        }
    };
    if task_statuses.is_empty() {
        let json_value = json!({
            "path": path,
            "status": "Not found",
        });
        let status = serde_json::to_string_pretty(&json_value).unwrap_or_else(
            |e| {
                error!("Error formatting to JSON: {:?}", e);
                format!("Error formatting to JSON: {:?}", e)
            }
        );
        debug!("{status:#?}");
        task_statuses.push(status);
    };
    let tasks_output = task_statuses.join("\n");
    info!("{tasks_output:#?}");

    (StatusCode::OK, format!(r#"{{
        Ready: {},
        Tasks: {},
    }}
    "#,
        ready_output,
        tasks_output,
    ))
}

async fn hash_handler(
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let file_path = match query_params.get("path") {
        None => "".to_string(),
        Some(file_path) => { file_path.to_owned() },
    };
    if file_path.is_empty() {
        return (StatusCode::BAD_REQUEST, String::from("'Path' parameter is missing. Set the requested 'path' first.\n"))
    };
    info!("File path: {:?}", file_path);

    let results = state.results.lock().await;
    match results.get(&file_path) {
        Some(hash) => {
            let json_value = json!({
                "file_path": file_path,
                "sha3_hash": hex::encode(hash),
            });
            (StatusCode::OK, serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            ))
        },
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

async fn proof_handler(
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let file_path = match query_params.get("path") {
        None => "".to_string(),
        Some(file_path) => { file_path.to_owned() },
    };
    if file_path.is_empty() {
        return (StatusCode::BAD_REQUEST, String::from("'Path' parameter is missing. Set the requested 'path' first.\n"))
    };
    info!("File path: {:?}", file_path);

    let app_cache = state.app_cache.read().clone().att_data;
    match app_cache.get(&file_path) {
        Some(att_data) => {
            let json_value = json!({
                "file_path": att_data.file_path,
                "sha3_hash": att_data.sha3_hash,
                "vrf_proof": att_data.vrf_proof,
                "vrf_cipher_suite": att_data.vrf_cipher_suite.to_string(),
            });
            (StatusCode::OK, serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            ))
        },
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

async fn doc_handler(
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let file_path = match query_params.get("path") {
        None => "".to_string(),
        Some(file_path) => { file_path.to_owned() },
    };
    if file_path.is_empty() {
        return (StatusCode::BAD_REQUEST, String::from("'Path' parameter is missing. Set the requested 'path' first.\n"))
    };
    info!("File path: {:?}", file_path);

    let view = query_params.get("view").unwrap_or(&String::from("json_hex")).to_owned();
    info!("View: {:?}", view);

    let app_cache = state.app_cache.read().clone().att_data;
    match app_cache.get(&file_path) {
        Some(att_data) => {
            let att_doc_fmt = att_doc_fmt(att_data.att_doc.as_slice(), view.as_str());
            let json_value = json!({
                    "file_path": att_data.file_path,
                    "sha3_hash": att_data.sha3_hash,
                    "vrf_proof": att_data.vrf_proof,
                    "vrf_cipher_suite": att_data.vrf_cipher_suite.to_string(),
                    "att_doc": att_doc_fmt,
                });
            (StatusCode::OK, serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            ))
        },
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

async fn hashes(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let path = match query_params.get("path") {
        None => "".to_string(),
        Some(path) => { path.to_owned() },
    };
    if path.is_empty() {
        return (StatusCode::BAD_REQUEST, String::from("'Path' parameter is missing. Set the requested 'path' first.\n"))
    };
    info!("Path: {:?}", path);

    let hashes = server_state.results.lock().await;
    let response = hashes.iter()
        .filter(
            |(key, _)|
                key.contains(path.as_str())
        ).map(
            |(path, hash)| {
                let json_value = json!({
                    "file_path": path,
                    "sha3_hash": hex::encode(hash.as_slice()),
                });
                let output = serde_json::to_string_pretty(&json_value).unwrap_or_else(
                    |e| {
                        error!("Error formatting to JSON: {:?}", e);
                        format!("Error formatting to JSON: {:?}", e)
                    }
                );
                info!("{output:#?}");
                output
            }
        )
        .collect::<Vec<String>>()
        .join("\n");
    info!("{response:#?}");

    (StatusCode::OK, format!(r#"{}"#, response))
}

async fn proofs(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let path = match query_params.get("path") {
        None => "".to_string(),
        Some(path) => { path.to_owned() },
    };
    if path.is_empty() {
        return (StatusCode::BAD_REQUEST, String::from("'Path' parameter is missing. Set the requested 'path' first.\n"))
    };
    info!("Path: {:?}", path);

    let app_cache = server_state.app_cache.read();
    let response = app_cache.att_data.iter()
        .filter(
            |(key, _)|
                key.contains(path.as_str())
        ).map(
            |(path, att_data)| {
                let json_value = json!({
                    "file_path": path,
                    "sha3_hash": att_data.sha3_hash,
                    "vrf_proof": att_data.vrf_proof,
                    "vrf_cipher_suite": att_data.vrf_cipher_suite.to_string(),
                }).to_string();
                let output = serde_json::to_string_pretty(&json_value).unwrap_or_else(
                    |e| {
                        error!("Error formatting to JSON: {:?}", e);
                        format!("Error formatting to JSON: {:?}", e)
                    }
                );
                info!("{output:#?}");
                output
            }
        )
        .collect::<Vec<String>>()
        .join("\n");
    info!("{response:#?}");

    (StatusCode::OK, format!(r#"{}"#, response))
}

async fn docs(
    Query(query_params): Query<HashMap<String, String>>,
    State(server_state): State<Arc<ServerState>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let path = match query_params.get("path") {
        None => "".to_string(),
        Some(path) => { path.to_owned() },
    };
    if path.is_empty() {
        return (StatusCode::BAD_REQUEST, String::from("'Path' parameter is missing. Set the requested 'path' first.\n"))
    };
    info!("Path: {:?}", path);

    let view = query_params.get("view").unwrap_or(&String::from("json_hex")).to_owned();
    info!("View: {:?}", view);

    let app_cache = server_state.app_cache.read();
    let response = app_cache.att_data.iter()
        .filter(
            |(key, _)|
                key.contains(path.as_str())
        ).map(
            |(path, att_data)| {
                let att_doc_fmt = att_doc_fmt(att_data.att_doc.as_slice(), view.as_str());
                let json_value = json!({
                    "file_path": path,
                    "sha3_hash": att_data.sha3_hash,
                    "vrf_proof": att_data.vrf_proof,
                    "vrf_cipher_suite": att_data.vrf_cipher_suite.to_string(),
                    "att_doc": att_doc_fmt,
                });
                let output = serde_json::to_string_pretty(&json_value).unwrap_or_else(
                    |e| {
                        error!("Error formatting to JSON: {:?}", e);
                        format!("Error formatting to JSON: {:?}", e)
                    }
                );
                info!("{output:#?}");
                output
            }
        )
        .collect::<Vec<String>>()
        .join("\n");
    info!("{response:#?}");

    (StatusCode::OK, format!(r#"{}"#, response))
}

async fn pubkeys(
    Query(query_params): Query<HashMap<String, String>>,
    State(app_state): State<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    info!("{query_params:?}");

    let view = query_params.get("view").unwrap_or(&String::from("hex")).to_owned();
    info!("View: {:?}", view);

    let fmt = query_params.get("fmt").unwrap_or(&String::from("pem")).to_owned();
    info!("Key Format: {:?}", fmt);

    let app_state = app_state.read().clone();

    let cipher = app_state.vrf_cipher_suite.to_nid();

    // SKey & PKey for proofs

    let skey4proofs_bytes = app_state.sk4proofs;
    let skey4proofs_pkey = PKey::private_key_from_pem(skey4proofs_bytes.as_slice()).unwrap();
    let skey4proofs_eckey = skey4proofs_pkey.ec_key().unwrap();
    let skey4proofs_bignum = skey4proofs_eckey.private_key().to_owned().unwrap();
    let _skey4proofs_vec = skey4proofs_bignum.to_vec();

    let alg = openssl::ec::EcGroup::from_curve_name(cipher).unwrap();
    let skey4proofs_ec_pubkey = openssl::ec::EcKey::from_public_key(&alg, skey4proofs_eckey.public_key()).unwrap();
    let skey4proofs_pkey_pubkey = PKey::from_ec_key(skey4proofs_ec_pubkey).unwrap();
    let skey4proofs_pubkey = match fmt.as_str() {
        "pem" => skey4proofs_pkey_pubkey.public_key_to_pem().unwrap_or_else(
            |e| {
                error!("Error while converting PKey for proofs into PEM format: {:?}", e);
                vec![]
            }
        ),
        "der" => skey4proofs_pkey_pubkey.public_key_to_der().unwrap_or_else(
            |e| {
                error!("Error while converting PKey for proofs into DER format: {:?}", e);
                vec![]
            }
        ),
        _ => skey4proofs_pkey_pubkey.public_key_to_pem().unwrap_or_else(
            |e| {
                error!("Error while converting PKey for proofs into PEM format: {:?}", e);
                vec![]
            }
        ),
    };

    let skey4proofs_pubkey_hex_string = hex::encode(skey4proofs_pubkey.clone());
    let skey4proofs_pubkey_string = String::from_utf8(skey4proofs_pubkey.clone()).unwrap_or_else(
        |e| {
            error!("Error while parsing PKey for proofs into string: {:?}", e);
            "".to_string()
        }
    );

    // SKey & PKey for docs

    let skey4docs_bytes = app_state.sk4docs;
    let skey4docs_pkey = PKey::private_key_from_pem(skey4docs_bytes.as_slice()).unwrap();
    let skey4docs_eckey = skey4docs_pkey.ec_key().unwrap();
    let skey4docs_bignum = skey4docs_eckey.private_key().to_owned().unwrap();
    let _skey4docs_vec = skey4docs_bignum.to_vec();

    let alg = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::SECP521R1).unwrap();
    let skey4docs_ec_pubkey = openssl::ec::EcKey::from_public_key(&alg, skey4docs_eckey.public_key()).unwrap();
    let skey4docs_pkey_pubkey = PKey::from_ec_key(skey4docs_ec_pubkey).unwrap();
    let skey4docs_pubkey = match fmt.as_str() {
        "pem" => skey4docs_pkey_pubkey.public_key_to_pem().unwrap_or_else(
            |e| {
                error!("Error while converting PKey for docs into PEM format: {:?}", e);
                vec![]
            }
        ),
        "der" => skey4docs_pkey_pubkey.public_key_to_pem().unwrap_or_else(
            |e| {
                error!("Error while converting PKey for docs into DER format: {:?}", e);
                vec![]
            }
        ),
        _ => skey4docs_pkey_pubkey.public_key_to_pem().unwrap_or_else(
            |e| {
                error!("Error while converting PKey for docs into PEM format: {:?}", e);
                vec![]
            }
        ),
    };

    let skey4docs_pubkey_hex_string = hex::encode(skey4docs_pubkey.clone());
    let skey4docs_pubkey_string = String::from_utf8(skey4docs_pubkey.clone()).unwrap_or_else(
        |e| {
            error!("Error while parsing PKey for docs into string: {:?}", e);
            "".to_string()
        }
    );

    match view.as_str() {
        "hex" | "json" => {
            let json_value = json!({
                "pubkey4proofs": skey4proofs_pubkey_hex_string,
                "pubkey4docs": skey4docs_pubkey_hex_string,
            });
            (StatusCode::OK, serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            ))
        },
        "string" | "text" => (StatusCode::OK, format!(r#"{{
            "pubkey4proofs": {},
            "pubkey4docs": {},
            }}
            "#,
            skey4proofs_pubkey_string, skey4docs_pubkey_string,
        )),
        _ => {
            let json_value = json!({
                "pubkey4proofs": skey4proofs_pubkey_hex_string,
                "pubkey4docs": skey4docs_pubkey_hex_string,
            });
            (StatusCode::OK, serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            ))
        },
    }
}

async fn nsm_desc(
    Query(query_params): Query<HashMap<String, String>>,
    State(app_state): State<Arc<RwLock<AppState>>>
) -> impl IntoResponse {
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
        "NSM description: [ major: {}, minor: {}, patch: {}, module_id: {}, max_pcrs: {}, locked_pcrs: {:?}, digest: {} ]\n",
        description.version_major,
        description.version_minor,
        description.version_patch,
        description.module_id,
        description.max_pcrs,
        description.locked_pcrs,
        LocalNsmDigest(description.digest)
    );

    (StatusCode::OK, format!(
        "NSM description: [ major: {}, minor: {}, patch: {}, module_id: {}, max_pcrs: {}, locked_pcrs: {:?}, digest: {} ]\n",
        description.version_major,
        description.version_minor,
        description.version_patch,
        description.module_id,
        description.max_pcrs,
        description.locked_pcrs,
        LocalNsmDigest(description.digest)
    ))
}

async fn rng_seq(
    Query(query_params): Query<HashMap<String, String>>,
    State(app_state): State<Arc<RwLock<AppState>>>
) -> impl IntoResponse {
    info!("{query_params:?}");
    let fd = app_state.read().nsm_fd;
    let length = query_params.get("length");
    let randomness_sequence = if let Some(length) = length {
        let len = length.to_owned().parse::<usize>().unwrap_or_else(|_| 512);
        get_randomness_sequence(fd, len)
    } else { get_randomness_sequence(fd, 512) };

    (StatusCode::OK, format!(r#"{}"#, hex::encode(randomness_sequence)))
}

fn att_doc_fmt(
    att_doc: &[u8],
    view: &str,
) -> String {
    let cose_doc = CoseSign1::from_bytes(att_doc).unwrap();
    let (protected_header, attestation_doc_bytes) =
        cose_doc.get_protected_and_payload::<Openssl>(None).unwrap();
    info!("Protected header: {:#?}", protected_header);
    let unprotected_header = cose_doc.get_unprotected();
    info!("Unprotected header: {:#?}", unprotected_header);
    let attestation_doc = AttestationDoc::from_binary(&attestation_doc_bytes[..]).unwrap();
    info!("Attestation document: {:#?}", attestation_doc);
    let attestation_doc_signature = cose_doc.get_signature();
    info!("Attestation document signature: {:#?}", hex::encode(attestation_doc_signature.clone()));

    let attestation_doc_json_string = serde_json::to_string_pretty(&attestation_doc).unwrap_or("".to_string());

    let att_doc_user_data_bytes = attestation_doc.clone().user_data.unwrap_or(ByteBuf::new()).into_vec();
    let att_doc_user_data = serde_json::from_slice::<AttUserData>(att_doc_user_data_bytes.as_slice()).unwrap();
    let att_doc_user_data_json_string = serde_json::to_string_pretty(&att_doc_user_data).unwrap_or("".to_string());

    let header_protected_str = protected_header.into_inner().iter().map(
        |(key, val)|
            format!(r#"{}: {}"#, hex::encode(serde_cbor::to_vec(key).unwrap()), hex::encode(serde_cbor::to_vec(val).unwrap()))
    )
    .collect::<Vec<String>>()
    .join(",\n");

    let header_unprotected_str = unprotected_header.into_inner().iter().map(
        |(key, val)|
            format!(r#"{}: {}"#, hex::encode(serde_cbor::to_vec(key).unwrap()), hex::encode(serde_cbor::to_vec(val).unwrap()))
    )
    .collect::<Vec<String>>()
    .join(",\n");

    let pcrs_fmt = attestation_doc.pcrs.iter().map(
        |(key, val)| format!(r#"{}: {}"#, key.to_string(), hex::encode(val.clone().into_vec()))
    )
    .collect::<Vec<String>>()
    .join(",\n");

    let cabundle_fmt = attestation_doc.cabundle.iter().map(
        |item| format!(r#"{}"#, hex::encode(item.clone().into_vec()))
    )
    .collect::<Vec<String>>()
    .join(",\n");

    let output =  match view {
        "bin" | "hex" | "bin_hex" | "cose_doc" | "cose_doc_bin" | "cose_doc_hex" | "cose_doc_bin_hex" =>
            hex::encode(att_doc),

        "att_doc" | "att_doc_bin" | "att_doc_hex" | "att_doc_bin_hex" =>
            hex::encode(attestation_doc_bytes.clone()),

        "att_doc_user_data" | "att_doc_user_data_json" | "att_doc_user_data_json_hex" => {
            let json_value = json!({
                "user_data": hex::encode(att_doc_user_data_bytes.clone()),
                "public_key": hex::encode(attestation_doc.public_key.unwrap_or(ByteBuf::new()).into_vec()),
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            )
        },

        "pcr" | "pcrs" => {
            let json_value = json!({
                "PCRs": format!(r#"{{
                    {}
                }}"#, pcrs_fmt)
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            )
        },

        "json_hex" => {
            let json_value = json!({
                "protected_header": format!(r#"{{
                    {}
                }}"#, header_protected_str),
                "unprotected_header": format!(r#"{{
                    {}
                }}"#, header_unprotected_str),
                "payload": {
                    "module_id": attestation_doc.module_id,
                    "digest": LocalNsmDigest(attestation_doc.digest).to_string(),
                    "timestamp": attestation_doc.timestamp.to_string(),
                    "PCRs": format!(r#"{{
                        {}
                    }}"#, pcrs_fmt),
                    "certificate": hex::encode(attestation_doc.certificate.into_vec()),
                    "ca_bundle": [
                        cabundle_fmt
                    ],
                    "public_key": hex::encode(attestation_doc.public_key.unwrap_or(ByteBuf::new()).into_vec()),
                    "user_data": att_doc_user_data_json_string,
                    "nonce": hex::encode(attestation_doc.nonce.unwrap_or(ByteBuf::new()).into_vec()),
                },
                "signature": hex::encode(attestation_doc_signature.clone()),
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            )
        },

        "json_str" => {
            let json_value = json!({
                "protected_header": format!(r#"{{
                    {}
                }}"#, header_protected_str),
                "unprotected_header": format!(r#"{{
                    {}
                }}"#, header_unprotected_str),
                "payload": attestation_doc_json_string,
                "signature": hex::encode(attestation_doc_signature.clone()),
            });
            serde_json::to_string_pretty(&json_value).unwrap_or_else(
                |e| {
                    error!("Error formatting to JSON: {:?}", e);
                    format!("Error formatting to JSON: {:?}", e)
                }
            )
        },

        "json_debug" => format!(r#"{{
            "protected_header": {{
                {:#?}
            }},
            "unprotected_header": {{
                {:#?}
            }},
            "payload": {},
            "signature": {:#?},
        }}
        "#,
            protected_header,
            unprotected_header,
            attestation_doc_json_string,
            attestation_doc_signature.clone(),
        ),

        "debug" => format!(r#"{:#?}"#, cose_doc),

        "debug_pretty_print" => format!(r#"{{
            "protected_header": {{
                {:#?}
            }},
            "unprotected_header": {{
                {:#?}
            }},
            "payload": {{
                "module_id": {:#?},
                "digest": {},
                "timestamp": {:#?},
                "PCRs": {{
                    {:#?}
                }},
                "certificate": {:#?},
                "ca_bundle\": [
                    {:#?}
                ],
                "public_key": {:#?},
                "user_data": {{
                    "file_path": {:#?},
                    "sha3_hash": {:#?},
                    "vrf_proof": {:#?},
                    "vrf_cipher_suite": {:#?},
                }},
                "nonce": {:#?},
            }},
            "signature": {:#?},
        }}
        "#,
            protected_header,
            unprotected_header,
            attestation_doc.module_id,
            LocalNsmDigest(attestation_doc.digest),
            attestation_doc.timestamp.to_string(),
            attestation_doc.pcrs,
            attestation_doc.certificate,
            attestation_doc.cabundle,
            attestation_doc.public_key,
            att_doc_user_data.file_path,
            att_doc_user_data.sha3_hash,
            att_doc_user_data.vrf_proof,
            att_doc_user_data.vrf_cipher_suite.to_string(),
            attestation_doc.nonce,
            attestation_doc_signature,
        ),

        _ => format!(r#"
            Attestation document ('bin_hex' string):
            {:#?}

            Set the 'view' format string parameter for attestation document:
            'view=(bin_hex | json_hex | json_str | json_debug | debug | debug_pretty_print)'
        "#,
            hex::encode(att_doc)
        ),
    };
    output
}

use std::str::FromStr;

#[derive(Debug, Clone)]
struct LocalNsmDigest(NsmDigest);

impl FromStr for LocalNsmDigest {
    type Err = CoseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SHA256" => Ok(LocalNsmDigest(NsmDigest::SHA256)),
            "SHA384" => Ok(LocalNsmDigest(NsmDigest::SHA384)),
            "SHA512" => Ok(LocalNsmDigest(NsmDigest::SHA512)),
            name => Err(CoseError::UnsupportedError(format!(
                "Algorithm '{}' is not supported",
                name
            ))),
        }
    }
}

impl std::fmt::Display for LocalNsmDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.0 {
            NsmDigest::SHA256 => "SHA256",
            NsmDigest::SHA384 => "SHA384",
            NsmDigest::SHA512 => "SHA512",
        };
        write!(f, "{}", name)
    }
}

async fn verify_hash(
    State(_app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyHashRequest>,
) -> impl IntoResponse {
    info!("{payload:?}");

    let path_string = payload.file_path;
    let request_hash_string = payload.sha3_hash;

    let path = StdPath::new(&path_string);

    // Check if the file path exists
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                format!("File path not found: {:?}\n", e),
            )
        }
    };

    // Check if the path is a directory
    if metadata.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            format!(r#"'file_path' field in a JSON request is a directory. Should be a file.
                'file_path' string from JSON request: {:?}
                'sha3_hash' string from JSON request: {:?}
            "#,
                path_string,
                request_hash_string,
            ),
        )
    };

    let hash = hash_file(path_string.as_str()).unwrap();
    let file_hash_string = hex::encode(hash.as_slice());

    if file_hash_string == request_hash_string {
        (
            StatusCode::OK,
            format!(r#"File present in FS and hash provided in JSON request is equal to actual file hash. Hash is VALID!
                'file_path' string from JSON request: {:?}
                'sha3_hash' string from JSON request: {:?}
                computed actual 'sha3_hash' string for file: {:?}
            "#,
                path_string,
                request_hash_string,
                file_hash_string,
            ),
        )
    } else {
        (
            StatusCode::OK,
            format!(r#"File present in FS, but hash provided in JSON request is NOT equal to actual file hash - hashes are different! Hash is INVALID!
                'file_path' string from JSON request: {:?}
                'sha3_hash' string from JSON request: {:?}
                computed actual 'sha3_hash' string for file: {:?}
            "#,
                path_string,
                request_hash_string,
                file_hash_string,
            ),
        )
    }
}

/// PCRs verification endpoint to compare base image PCR registers with actual running enclave PCR registers
async fn verify_pcrs(
    State(app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyPCRsRequest>,
) -> impl IntoResponse {
    info!("{payload:?}");
    let pcrs_string_req = payload.pcrs;

    let fd = app_state.read().nsm_fd;
    let cose_doc_bytes = get_attestation_doc(fd, None, None, None);

    let cose_doc = CoseSign1::from_bytes(cose_doc_bytes.as_slice()).unwrap();
    let (protected_header, attestation_doc_bytes) =
        cose_doc.get_protected_and_payload::<Openssl>(None).unwrap();
    info!("Protected header: {:#?}", protected_header);
    let unprotected_header = cose_doc.get_unprotected();
    info!("Unprotected header: {:#?}", unprotected_header);
    let attestation_doc = AttestationDoc::from_binary(&attestation_doc_bytes[..]).unwrap();
    info!("Attestation document: {:#?}", attestation_doc);
    let attestation_doc_signature = cose_doc.get_signature();
    info!("Attestation document signature: {:#?}", hex::encode(attestation_doc_signature.clone()));

    let pcrs_fmt = attestation_doc.pcrs.iter().map(
        |(key, val)| format!(r#"{}: {}"#, key.to_string(), hex::encode(val.clone().into_vec()))
    )
    .collect::<Vec<String>>()
    .join(",\n");

    if pcrs_string_req == pcrs_fmt {
        (
            StatusCode::OK,
            format!(r#"PCRs provided in JSON request are equal to actual PCRs retrieved from enclave's attestation document.
                PCR registers of base image and running enclave from base image are equal and are VALID!
                PCRs from JSON request: {:?}
                PCRs retrieved from enclave's attestation document: {:?}
            "#,
                pcrs_string_req,
                pcrs_fmt,
            ),
        )
    } else {
        (
            StatusCode::OK,
            format!(r#"PCRs provided in JSON request are NOT equal to actual PCRs retrieved from enclave's attestation document.
                PCR registers of base image and running enclave from base image are NOT equal and are INVALID!
                PCRs from JSON request: {:?}
                PCRs retrieved from enclave's attestation document: {:?}
            "#,
                pcrs_string_req,
                pcrs_fmt,
            ),
        )
    }
}

async fn verify_proof(
    State(_app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyProofRequest>,
) -> impl IntoResponse {
    let att_doc_user_data_bytes = match hex::decode(payload.user_data) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Malformed 'user_data' input as a JSON field: {}\n
            Please use GET 'att_doc_user_data' endpoint to request correct JSON", e);
            return (StatusCode::BAD_REQUEST, format!("Malformed 'user_data' input as a JSON field: {}\n
            Please use GET 'att_doc_user_data' endpoint to request correct JSON", e))
        }
    };

    let att_doc_user_data = match serde_json::from_slice::<AttUserData>(att_doc_user_data_bytes.as_slice()) {
        Ok(user_data) => user_data,
        Err(e) => {
            error!("Malformed 'AttUserData' data structure from 'user_data' input as a JSON field: {}\n
            Please use GET 'att_doc_user_data' endpoint to request correct JSON", e);
            return (StatusCode::BAD_REQUEST, format!("Malformed 'AttUserData' data structure from 'user_data' input as a JSON field: {}\n
            Please use GET 'att_doc_user_data' endpoint to request correct JSON", e))
        }
    };

    let pubkey4proof_pubkey_bytes = match hex::decode(payload.public_key) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Malformed 'public_key' input as a JSON field: {}\n
            Please use GET 'att_doc_user_data' endpoint to request correct JSON", e);
            return (StatusCode::BAD_REQUEST, format!("Malformed 'public_key' input as a JSON field: {}\n
            Please use GET 'att_doc_user_data' endpoint to request correct JSON", e))
        }
    };

    let vrf_proof_bytes = match hex::decode(att_doc_user_data.vrf_proof) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Malformed 'vrf_proof' input as field of 'AttUserData' data structure from 'user_data' input as a JSON field: {}\n
            Please use GET 'att_doc_user_data' endpoint to request correct JSON", e);
            return (StatusCode::BAD_REQUEST, format!("Malformed 'vrf_proof' input as field of 'AttUserData' data structure from 'user_data' input as a JSON field: {}\n
            Please use GET 'att_doc_user_data' endpoint to request correct JSON", e))
        }
    };

    // VRF Proof hash based on private key generated for file path and file hash pair (emulation of CoW FS metadata for enclave's ramdisk FS)
    let att_proof_data = AttProofData{
        file_path: att_doc_user_data.file_path.clone(),
        sha3_hash: hex::encode(att_doc_user_data.sha3_hash.clone()),
    };
    let att_proof_data_json_bytes = serde_json::to_vec(&att_proof_data).unwrap();

    let cipher_suite = att_doc_user_data.vrf_cipher_suite;

    match vrf_verify(att_proof_data_json_bytes.as_slice(), vrf_proof_bytes.as_slice(), pubkey4proof_pubkey_bytes.as_slice(), cipher_suite) {
        Ok(message) => (StatusCode::OK, format!("{:?}\n", message)),
        Err(message) => (StatusCode::OK, format!("{:?}\n", message)),
    }
}

async fn verify_doc(
    State(_app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyDocRequest>,
) -> impl IntoResponse {
    let cose_doc_bytes = match hex::decode(payload.cose_doc_bytes) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Malformed 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e);
            return (StatusCode::BAD_REQUEST, format!("Malformed 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e))
        }
    };

    let cose_doc = CoseSign1::from_bytes(cose_doc_bytes.as_slice()).unwrap();
    let (protected_header, attestation_doc_bytes) =
        cose_doc.get_protected_and_payload::<Openssl>(None).unwrap();
    info!("Protected header: {:#?}", protected_header);
    let unprotected_header = cose_doc.get_unprotected();
    info!("Unprotected header: {:#?}", unprotected_header);
    let attestation_doc = AttestationDoc::from_binary(&attestation_doc_bytes[..]).unwrap();
    info!("Attestation document: {:#?}", attestation_doc);
    let attestation_doc_signature = cose_doc.get_signature();
    info!("Attestation document signature: {:#?}", hex::encode(attestation_doc_signature.clone()));

    let att_doc_certificate_bytes = attestation_doc.certificate.into_vec();
    let att_doc_certificate = match X509::from_der(att_doc_certificate_bytes.as_slice()) {
        Ok(x509) => x509,
        Err(e) => {
            error!("Malformed 'certificate' in attestation document, incorrect 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e);
            return (StatusCode::BAD_REQUEST, format!("Malformed 'certificate' in attestation document, incorrect 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e))
        }
    };
    let att_doc_certificate_pubkey_pkey = att_doc_certificate.public_key().unwrap();

    let outcome = cose_doc.verify_signature::<Openssl>(&att_doc_certificate_pubkey_pkey);
    match outcome {
        Ok(result) => if result {
            (StatusCode::OK, format!(r#"
                Attestation document signature verification: {:?}
                Attestation document signature is VALID!
                Attestation document signature verification against attestation document certificate public key is successful!
                "#, "Successful".to_string()))
        } else {
            (StatusCode::OK, format!(r#"
                Attestation document signature verification: {:?}
                Attestation document signature is INVALID!
                Attestation document signature verification against attestation document certificate public key is NOT successful!
                "#, "NOT successful".to_string()))
        },
        Err(error) => (StatusCode::BAD_REQUEST, format!("Verification failed. An error returned during attestation document signature verification check: {:?}", error))
    }
}

fn check_cert_validity(cert: &X509) -> Result<bool, String> {
    // Check if the certificate has expired or is not yet valid
    // let now = Asn1Time::days_from_now(0).unwrap();
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    let seconds_since_epoch = since_epoch.as_secs() as i64;
    let now = Asn1Time::from_unix(seconds_since_epoch).unwrap();

    let not_before = match cert.not_before().compare(&now) {
        Ok(ord) => if ord.is_gt() { true } else { false },
        Err(e) => return Err(format!("Certificate is not yet valid. {:?}", e)),
    };

    let not_after = match cert.not_after().compare(&now) {
        Ok(ord) => if ord.is_lt() { true } else { false },
        Err(e) => return Err(format!("Certificate has expired. {:?}", e)),
    };

    // Additional checks can be performed here, such as checking the certificate's subject and issuer

    if not_before == false || not_after == false {
        Ok(false)
    } else {
        Ok(true)
    }
}

async fn verify_cert_valid(
    State(_app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyDocRequest>,
) -> impl IntoResponse {
    let cose_doc_bytes = match hex::decode(payload.cose_doc_bytes) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Malformed 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e);
            return (StatusCode::BAD_REQUEST, format!("Malformed 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e))
        }
    };

    let cose_doc = CoseSign1::from_bytes(cose_doc_bytes.as_slice()).unwrap();
    let (protected_header, attestation_doc_bytes) =
        cose_doc.get_protected_and_payload::<Openssl>(None).unwrap();
    info!("Protected header: {:#?}", protected_header);
    let unprotected_header = cose_doc.get_unprotected();
    info!("Unprotected header: {:#?}", unprotected_header);
    let attestation_doc = AttestationDoc::from_binary(&attestation_doc_bytes[..]).unwrap();
    info!("Attestation document: {:#?}", attestation_doc);
    let attestation_doc_signature = cose_doc.get_signature();
    info!("Attestation document signature: {:#?}", hex::encode(attestation_doc_signature.clone()));

    let att_doc_certificate_bytes = attestation_doc.certificate.into_vec();
    let att_doc_certificate = match X509::from_der(att_doc_certificate_bytes.as_slice()) {
        Ok(x509) => x509,
        Err(e) => {
            error!("Malformed 'certificate' in attestation document, incorrect 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e);
            return (StatusCode::BAD_REQUEST, format!("Malformed 'certificate' in attestation document, incorrect 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e))
        }
    };

    let att_doc_cert_info = att_doc_certificate.to_text().unwrap();
    let att_doc_cert_info_string = String::from_utf8_lossy(att_doc_cert_info.as_slice());
    info!("Attestation document certificate information: {}\n", att_doc_cert_info_string);

    let att_doc_cert_pubkey_pkey = att_doc_certificate.public_key().unwrap();
    let att_doc_cert_verification_result = match att_doc_certificate.verify(&att_doc_cert_pubkey_pkey) {
        Ok(valid) => if !valid {
            error!("Attestation document certificate signature verification against its public key is NOT successful, signature is INVALID! Certificate information: {}\n", att_doc_cert_info_string);
            Err(format!("Attestation document certificate signature verification against its public key is NOT successful, signature is INVALID! Certificate information: {}\n", att_doc_cert_info_string))
        } else {
            Ok(format!("Attestation document certificate signature verification against its public key is successful, signature is VALID! Certificate information: {}\n", att_doc_cert_info_string))
        },
        Err(e) => {
            error!("Attestation document certificate signature verification against its public key FAILED! Error returned: {:?}\nCertificate information: {}\n", e, att_doc_cert_info_string);
            Err(format!("Attestation document certificate signature verification against its public key FAILED! Error returned: {:?}\nCertificate information: {}\n", e, att_doc_cert_info_string))
        },
    };

    let att_doc_cert_validity_check_result = match check_cert_validity(&att_doc_certificate) {
        Ok(valid) => if !valid {
            error!("Attestation document certificate is not yet valid or expired. {}\n", att_doc_cert_info_string);
            Err(format!("Attestation document certificate is not yet valid or expired. {}\n", att_doc_cert_info_string))
        } else {
            Ok(format!("Attestation document certificate validity check (validation) is SUCCESSFUL! Certificate is VALID! Certificate information: {}\n", att_doc_cert_info_string))
        },
        Err(e) => {
            error!("Attestation document certificate validity check (validation) FAILED! Error returned: {:?}\nCertificate information: {}\n", e, att_doc_cert_info_string);
            Err(format!("Attestation document certificate validity check (validation) FAILED! Error returned: {:?}\nCertificate information: {}\n", e, att_doc_cert_info_string))
        },
    };

    match (att_doc_cert_verification_result, att_doc_cert_validity_check_result) {
        (Ok(verification_result), Ok(validity_check_result)) =>
            (StatusCode::OK, format!(r#"
                Attestation document certificate signature verification result:
                    {:?}
                Attestation document certificate validity check (validation) result:
                    {:?}
                "#,
                    verification_result,
                    validity_check_result,
            )),
        (Err(verification_result), Ok(validity_check_result)) |
        (Ok(verification_result), Err(validity_check_result)) |
        (Err(verification_result), Err(validity_check_result)) =>
            (StatusCode::BAD_REQUEST, format!(r#"
                Attestation document certificate signature verification result:
                    {:?}
                Attestation document certificate validity check (validation) result:
                    {:?}
                "#,
                    verification_result,
                    validity_check_result,
            )),
    }
}

async fn verify_cert_bundle(
    State(_app_state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<VerifyDocRequest>,
) -> impl IntoResponse {
    let cose_doc_bytes = match hex::decode(payload.cose_doc_bytes) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Malformed 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e);
            return (StatusCode::BAD_REQUEST, format!("Malformed 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e))
        }
    };

    let cose_doc = CoseSign1::from_bytes(cose_doc_bytes.as_slice()).unwrap();
    let (protected_header, attestation_doc_bytes) =
        cose_doc.get_protected_and_payload::<Openssl>(None).unwrap();
    info!("Protected header: {:#?}", protected_header);
    let unprotected_header = cose_doc.get_unprotected();
    info!("Unprotected header: {:#?}", unprotected_header);
    let attestation_doc = AttestationDoc::from_binary(&attestation_doc_bytes[..]).unwrap();
    info!("Attestation document: {:#?}", attestation_doc);
    let attestation_doc_signature = cose_doc.get_signature();
    info!("Attestation document signature: {:#?}", hex::encode(attestation_doc_signature.clone()));

    if attestation_doc.cabundle.is_empty() {
        error!("Malformed 'cabundle' in attestation document, incorrect 'cose_doc_bytes' input as a JSON field: {}\n
        Please use GET 'cose_doc' endpoint to request correct JSON", "");
        return (StatusCode::BAD_REQUEST, format!("Malformed 'cabundle' in attestation document, incorrect 'cose_doc_bytes' input as a JSON field: {}\n
        Please use GET 'cose_doc' endpoint to request correct JSON", ""))
    };

    let att_doc_certificate_bytes = attestation_doc.certificate.into_vec();
    let att_doc_certificate = match X509::from_der(att_doc_certificate_bytes.as_slice()) {
        Ok(x509) => x509,
        Err(e) => {
            error!("Malformed 'certificate' in attestation document, incorrect 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e);
            return (StatusCode::BAD_REQUEST, format!("Malformed 'certificate' in attestation document, incorrect 'cose_doc_bytes' input as a JSON field: {}\n
            Please use GET 'cose_doc' endpoint to request correct JSON", e))
        }
    };

    let att_doc_cert_info = att_doc_certificate.to_text().unwrap();
    let att_doc_cert_info_string = String::from_utf8_lossy(att_doc_cert_info.as_slice());
    info!("Attestation document certificate information: {}\n", att_doc_cert_info_string);

    let att_doc_cert_pubkey_pkey = att_doc_certificate.public_key().unwrap();
    match att_doc_certificate.verify(&att_doc_cert_pubkey_pkey) {
        Ok(valid) => if !valid {
            error!("Attestation document certificate signature verification against its public key is not successful, signature is invalid. Certificate information: {}\n", att_doc_cert_info_string);
            return (StatusCode::BAD_REQUEST, format!("Attestation document certificate signature verification against its public key is not successful, signature is invalid. Certificate information: {}\n", att_doc_cert_info_string))
        },
        Err(e) => {
            error!("Attestation document certificate signature verification against its public key is not successful. Error returned: {:?}\nCertificate information: {}\n", e, att_doc_cert_info_string);
            return (StatusCode::BAD_REQUEST, format!("Attestation document certificate signature verification against its public key is not successful. Error returned: {:?}\nCertificate information: {}\n", e, att_doc_cert_info_string))
        },
    };

    match check_cert_validity(&att_doc_certificate) {
        Ok(valid) => if !valid {
            error!("Attestation document certificate is not yet valid or expired.\n");
            return (StatusCode::BAD_REQUEST, format!("Attestation document certificate is not yet valid or expired. {}\n", ""))
        },
        Err(e) => {
            error!("Attestation document certificate is not yet valid or expired. Error returned: {:?}\n", e);
            return (StatusCode::BAD_REQUEST, format!("Attestation document certificate is not yet valid or expired. Error returned: {:?}\n", e))
        },
    };

    let (root_cert, intermediate_certs) = attestation_doc.cabundle.split_first().unwrap();
    let root_cert = root_cert.to_vec();
    let root_cert_x509 = X509::from_der(root_cert.as_slice()).unwrap();

    let root_cert_info = root_cert_x509.to_text().unwrap();
    let root_cert_info_string = String::from_utf8_lossy(root_cert_info.as_slice());
    info!("Root certificate information: {}\n", root_cert_info_string);

    let root_cert_pubkey_pkey = root_cert_x509.public_key().unwrap();
    match root_cert_x509.verify(&root_cert_pubkey_pkey) {
        Ok(valid) => if !valid {
            error!("Attestation document root certificate from CA bundle chain signature verification against its public key is not successful, signature is invalid. Certificate information: {}\n", root_cert_info_string);
            return (StatusCode::BAD_REQUEST, format!("Attestation document root certificate from CA bundle chain signature verification against its public key is not successful, signature is invalid. Certificate information: {}\n", root_cert_info_string))
        },
        Err(e) => {
            error!("Attestation document root certificate from CA bundle chain signature verification against its public key is not successful. Error returned: {:?}\nCertificate information: {}\n", e, root_cert_info_string);
            return (StatusCode::BAD_REQUEST, format!("Attestation document root certificate from CA bundle chain signature verification against its public key is not successful. Error returned: {:?}\nCertificate information: {}\n", e, root_cert_info_string))
        },
    };

    match check_cert_validity(&root_cert_x509) {
        Ok(valid) => if !valid {
            error!("Attestation document root certificate from CA bundle is not yet valid or expired.\n");
            return (StatusCode::BAD_REQUEST, format!("Attestation document root certificate from CA bundle is not yet valid or expired. {}\n", ""))
        },
        Err(e) => {
            error!("Attestation document root certificate from CA bundle is not yet valid or expired. Error returned: {:?}\n", e);
            return (StatusCode::BAD_REQUEST, format!("Attestation document root certificate from CA bundle is not yet valid or expired. Error returned: {:?}\n", e))
        },
    };

    let mut root_cert_store_builder = X509StoreBuilder::new().unwrap();
    root_cert_store_builder.add_cert(root_cert_x509).unwrap();
    let root_cert_store = root_cert_store_builder.build();

    let mut intermediate_certs_stack = Stack::new().unwrap();
    for cert in intermediate_certs.iter() {
        let cert = cert.to_vec();
        let cert_x509 = X509::from_der(cert.as_slice()).unwrap();

        let cert_info = cert_x509.to_text().unwrap();
        let cert_info_string = String::from_utf8_lossy(cert_info.as_slice());
        info!("Intermediate certificate information: {}\n", cert_info_string);

        let cert_pubkey_pkey = cert_x509.public_key().unwrap();
        match cert_x509.verify(&cert_pubkey_pkey) {
            Ok(valid) => if !valid {
                error!("Attestation document intermediate certificate from CA bundle chain signature verification against its public key is not successful, signature is invalid. Certificate information: {}\n", cert_info_string);
                return (StatusCode::BAD_REQUEST, format!("Attestation document intermediate certificate from CA bundle chain signature verification against its public key is not successful, signature is invalid. Certificate information: {}\n", cert_info_string))
            },
            Err(e) => {
                error!("Attestation document intermediate certificate from CA bundle chain signature verification against its public key is not successful. Error returned: {:?}\nCertificate information: {}\n", e, cert_info_string);
                return (StatusCode::BAD_REQUEST, format!("Attestation document intermediate certificate from CA bundle chain signature verification against its public key is not successful. Error returned: {:?}\nCertificate information: {}\n", e, cert_info_string))
            },
        };

        match check_cert_validity(&cert_x509) {
            Ok(valid) => if !valid {
                error!("Attestation document intermediate certificate from CA bundle chain is not yet valid or expired. Certificate information: {}\n", cert_info_string);
                return (StatusCode::BAD_REQUEST, format!("Attestation document intermediate certificate from CA bundle chain is not yet valid or expired. Certificate information: {}\n", cert_info_string))
            },
            Err(e) => {
                error!("Attestation document intermediate certificate from CA bundle chain is not yet valid or expired. Error returned: {:?}\nCertificate information: {}\n", e, cert_info_string);
                return (StatusCode::BAD_REQUEST, format!("Attestation document intermediate certificate from CA bundle chain is not yet valid or expired. Error returned: {:?}\nCertificate information: {}\n", e, cert_info_string))
            },
        };

        intermediate_certs_stack.push(cert_x509).unwrap();
    };

    let mut ctx = X509StoreContext::new().unwrap();
    let outcome = ctx.init(&root_cert_store, &att_doc_certificate, &intermediate_certs_stack, |ctx| ctx.verify_cert());

    match outcome {
        Ok(result) => if result {
            (StatusCode::OK, format!(r#"
                Attestation document certificate verification: {:?}
                Attestation document certificate is VALID!
                Attestation document certificate verification against attestation document certificates bundle (root certificate and intermediate certificates) is successful!
                "#, "Successful".to_string()
            ))
        } else {
            let cert_verification_context = format!("OpenSSL error: {:?}", ctx.error().error_string());
            (StatusCode::OK, format!(r#"
                Attestation document certificate verification: {:?}
                Attestation document certificate is INVALID!
                Attestation document certificate verification against attestation document certificates bundle (root certificate and intermediate certificates) is NOT successful!
                Verification context: {:?}
                "#,
                    "NOT successful".to_string(),
                    cert_verification_context,
            ))
        },
        Err(error) => (StatusCode::BAD_REQUEST, format!("Verification failed. An error returned during attestation document certificate verification check: {:?}", error))
    }
}

/// Randomly generate PRIME256V1/P-256 key to use for validating signing internally
fn generate_ec256_keypair() -> (PKey<Private>, PKey<Public>) {
    let alg = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::X9_62_PRIME256V1).unwrap();
    let ec_private = openssl::ec::EcKey::generate(&alg).unwrap();
    let ec_public =
        openssl::ec::EcKey::from_public_key(&alg, ec_private.public_key()).unwrap();
    (
        PKey::from_ec_key(ec_private).unwrap(),
        PKey::from_ec_key(ec_public).unwrap(),
    )
}

/// Randomly generate SECP384R1/P-384 key to use for validating signing internally
fn generate_ec384_keypair() -> (PKey<Private>, PKey<Public>) {
    let alg = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::SECP384R1).unwrap();
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
    let alg = openssl::ec::EcGroup::from_curve_name(openssl::nid::Nid::SECP521R1).unwrap();
    let ec_private = openssl::ec::EcKey::generate(&alg).unwrap();
    let ec_public =
        openssl::ec::EcKey::from_public_key(&alg, ec_private.public_key()).unwrap();
    (
        PKey::from_ec_key(ec_private).unwrap(),
        PKey::from_ec_key(ec_public).unwrap(),
    )
}

fn generate_keypair(cipher_id: CipherID) -> (PKey<Private>, PKey<Public>) {
    let alg = openssl::ec::EcGroup::from_curve_name(cipher_id).unwrap();
    let ec_private = openssl::ec::EcKey::generate(&alg).unwrap();
    let ec_public =
        openssl::ec::EcKey::from_public_key(&alg, ec_private.public_key()).unwrap();
    (
        PKey::from_ec_key(ec_private).unwrap(),
        PKey::from_ec_key(ec_public).unwrap(),
    )
}

fn vrf_proof(message: &[u8], secret_key: &[u8], cipher_suite: CipherSuite) -> Result<Vec<u8>, Error> {
    let mut vrf  = ECVRF::from_suite(cipher_suite).unwrap();
    let _public_key = vrf.derive_public_key(&secret_key).unwrap();
    let proof = vrf.prove(&secret_key, &message).unwrap();
    Ok(proof)
}

fn vrf_verify(message: &[u8], proof: &[u8], public_key: &[u8], cipher_suite: CipherSuite) -> Result<String, String> {
    let mut vrf  = ECVRF::from_suite(cipher_suite).unwrap();
    let hash = vrf.proof_to_hash(&proof).unwrap();
    let outcome = vrf.verify(&public_key, &proof, &message);
    match outcome {
        Ok(outcome) => {
            let result = if hash == outcome {
                info!("VRF proof is valid!");
                Ok("VRF proof is valid!".to_string())
            } else {
                error!("VRF proof is not valid!");
                Err("VRF proof is not valid!".to_string())
            };
            result
        }
        Err(e) => {
            error!("VRF proof is not valid! Error: {:?}", e);
            Err(format!("VRF proof is not valid! Error: {:?}", e))
        }
    }
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

fn get_randomness_sequence(fd: i32, count_bytes: usize) -> Vec<u8> {
    let mut prev_random: Vec<u8> = vec![];
    let mut random: Vec<u8> = vec![];
    let mut random_bytes: Vec<u8> = vec![];
    let random_gen_cycles = 128; // for reliable non-equal randomness sequences generation

    while random_bytes.len() < count_bytes {
        for _ in 0..random_gen_cycles {
            random = match nsm_process_request(fd, Request::GetRandom) {
                Response::GetRandom { random } => {
                    assert!(!random.is_empty());
                    assert!(prev_random != random);
                    prev_random = random.clone();
                    info!("Random bytes: {:?}", random.clone());
                    random
                },
                resp => {
                    error!(
                        "GetRandom: expecting Response::GetRandom, but got {:?} instead",
                        resp
                    );
                    return vec![]
                },
            };
        };
        random_bytes.append(&mut random);
    };
    random_bytes[..count_bytes].to_vec()
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
            // info!("Attestation document: {:#?}", document);
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
    handle.graceful_shutdown(Some(Duration::from_secs(10))); // 10 secs are how long docker will wait to force shutdown
}
