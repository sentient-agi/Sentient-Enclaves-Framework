//! Remote attestation web-server for Sentient Enclaves Framework

use anyhow::{Context, Result};
use axum::routing::{get, post};
use axum::Router;
use axum_server::tls_openssl::OpenSSLConfig;
use parking_lot::RwLock;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use ra_web_srv::cipher::CipherMapper;
use ra_web_srv::config::{AppConfig, Keys, NATSMQPersistency};
use ra_web_srv::crypto::{generate_ec512_keypair, generate_keypair};
use ra_web_srv::handlers::*;
use ra_web_srv::nats::nats_orchestrator;
use ra_web_srv::server::{redirect_http_to_https, shutdown_signal};
use ra_web_srv::state::ServerState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Remote Attestation Web Server");

    // Load configuration (now using YAML)
    let default_config_path = format!("./.config/{}.config.yaml", env!("CARGO_CRATE_NAME"));
    let config_path = default_config_path.as_str();

    let app_config = Arc::new(
        AppConfig::new_from_file(config_path)
            .context(format!("Failed to load configuration from '{}'", config_path))?,
    );

    let ports = app_config.get_ports();
    info!(
        "Configured ports - HTTP: {}, HTTPS: {}",
        ports.http, ports.https
    );

    // Initialize NSM
    let fd = app_config
        .get_nsm_fd()
        .context("Failed to initialize NSM device")?;

    if fd < 0 {
        error!("NSM initialization returned invalid fd: {}", fd);
        return Err(anyhow::anyhow!("NSM initialization failed with fd: {}", fd));
    }
    info!("NSM device initialized with fd: {}", fd);

    // Get VRF cipher suite
    let vrf_cipher_suite = app_config
        .get_vrf_cipher_suite()
        .context("Failed to get VRF cipher suite from configuration")?;
    info!("VRF cipher suite: {}", vrf_cipher_suite.to_string());

    // Create server state
    let state = Arc::new(ServerState::new());

    // Initialize app state
    {
        let mut app_state = state.app_state.write();
        app_state.nsm_fd = fd;
        app_state.vrf_cipher_suite = vrf_cipher_suite;
    }

    // Setup keys for proofs
    let skey4proofs = app_config.get_keys().sk4proofs.unwrap_or_default();
    if skey4proofs.is_empty() {
        info!("Generating new key for VRF proofs");
        let cipher = app_config.get_vrf_cipher_suite()?.to_nid();
        let (skey, _pkey) =
            generate_keypair(cipher).context("Failed to generate keypair for proofs")?;

        let skey_bytes = skey
            .private_key_to_pem_pkcs8()
            .context("Failed to convert private key to PEM")?;
        info!("Generated SK for VRF Proofs: {} bytes", skey_bytes.len());

        state.app_state.write().sk4proofs = skey_bytes.clone();

        // Save key to file
        let skey_string =
            String::from_utf8(skey_bytes.clone()).context("Failed to convert key to string")?;
        std::fs::create_dir_all("./.keys/").context("Failed to create .keys directory")?;
        std::fs::write("./.keys/sk4proofs.pkcs8.pem", &skey_string)
            .context("Failed to write proofs key file")?;

        // Update config
        let skey_hex = hex::encode(&skey_bytes);
        app_config.update_keys(Keys {
            sk4proofs: Some(skey_hex),
            sk4docs: app_config.get_keys().sk4docs,
        });

        std::fs::create_dir_all("./.config/").context("Failed to create .config directory")?;
        app_config
            .save_to_file(config_path)
            .context("Failed to save configuration")?;
    } else {
        let skey_bytes =
            hex::decode(&skey4proofs).context("Failed to decode proofs key from hex")?;
        let skey_byte_len = skey_bytes.len();

        match skey_byte_len {
            237 | 241 | 384 => {
                info!(
                    "Using existing SK for VRF Proofs: {} bytes",
                    skey_byte_len
                );
            }
            _ => {
                error!(
                    "Invalid SK length for VRF Proofs: {} bytes",
                    skey_byte_len
                );
                return Err(anyhow::anyhow!(
                    "SK length for VRF Proofs mismatch: {}",
                    skey_byte_len
                ));
            }
        }

        state.app_state.write().sk4proofs = skey_bytes;
    }

    // Setup keys for docs
    let skey4docs = app_config.get_keys().sk4docs.unwrap_or_default();
    if skey4docs.is_empty() {
        info!("Generating new key for attestation documents signing");
        let (skey, _pkey) =
            generate_ec512_keypair().context("Failed to generate EC512 keypair for docs")?;

        let skey_bytes = skey
            .private_key_to_pem_pkcs8()
            .context("Failed to convert private key to PEM")?;
        info!(
            "Generated SK for attestation documents signing: {} bytes",
            skey_bytes.len()
        );

        state.app_state.write().sk4docs = skey_bytes.clone();

        // Save key to file
        let skey_string =
            String::from_utf8(skey_bytes.clone()).context("Failed to convert key to string")?;
        std::fs::create_dir_all("./.keys/").context("Failed to create .keys directory")?;
        std::fs::write("./.keys/sk4docs.pkcs8.pem", &skey_string)
            .context("Failed to write docs key file")?;

        // Update config
        let skey_hex = hex::encode(&skey_bytes);
        app_config.update_keys(Keys {
            sk4proofs: app_config.get_keys().sk4proofs,
            sk4docs: Some(skey_hex),
        });

        std::fs::create_dir_all("./.config/").context("Failed to create .config directory")?;
        app_config
            .save_to_file(config_path)
            .context("Failed to save configuration")?;
    } else {
        let skey_bytes = hex::decode(&skey4docs).context("Failed to decode docs key from hex")?;
        let skey_byte_len = skey_bytes.len();

        if skey_byte_len != 384 {
            error!(
                "Invalid SK length for attestation documents signing: {} bytes",
                skey_byte_len
            );
            return Err(anyhow::anyhow!(
                "SK length for attestation documents signing mismatch: {}",
                skey_byte_len
            ));
        }

        info!(
            "Using existing SK for attestation documents signing: {} bytes",
            skey_byte_len
        );
        state.app_state.write().sk4docs = skey_bytes;
    }

    // Setup NATS persistence if enabled
    let nats_config = app_config
        .get_nats_config()
        .unwrap_or_else(NATSMQPersistency::default);
    if nats_config
        .nats_persistency_enabled
        .is_some_and(|enabled| enabled > 0)
    {
        info!("NATS persistence enabled, starting orchestrator");
        let app_state_clone = Arc::clone(&state.app_state);
        let app_cache_clone = Arc::clone(&state.app_cache);
        tokio::spawn(async move {
            if let Err(e) = nats_orchestrator(app_state_clone, app_cache_clone, nats_config).await {
                error!("[NATS Orchestrator] Error: {}", e);
            }
        });
    } else {
        info!("NATS persistence disabled");
    }

    // Create TLS server handle
    let handle = axum_server::Handle::new();
    let shutdown_future = shutdown_signal(handle.clone(), Arc::clone(&state.app_state));

    // Spawn HTTP to HTTPS redirect server
    tokio::spawn(redirect_http_to_https(ports.clone(), shutdown_future));

    // Configure TLS
    let cert_dir = std::env::var("CERT_DIR").unwrap_or_else(|e| {
        debug!(
            "CERT_DIR env var not set ({}), using default './certs/'",
            e
        );
        "./certs/".to_string()
    });

    let tls_config = OpenSSLConfig::from_pem_file(
        PathBuf::from(&cert_dir).join("cert.pem"),
        PathBuf::from(&cert_dir).join("skey.pem"),
    )
    .context(format!(
        "Failed to load TLS certificates from '{}'",
        cert_dir
    ))?;

    // Build router
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
        .route(
            "/pubkeys/",
            get(pubkeys).with_state(Arc::clone(&state.app_state)),
        )
        .route("/pcrs/", get(get_pcrs))
        .route(
            "/verify_pcrs/",
            post(verify_pcrs).with_state(Arc::clone(&state.app_state)),
        )
        .route(
            "/verify_hash/",
            post(verify_hash).with_state(Arc::clone(&state.app_state)),
        )
        .route(
            "/verify_proof/",
            post(verify_proof).with_state(Arc::clone(&state.app_state)),
        )
        .route(
            "/verify_doc/",
            post(verify_doc).with_state(Arc::clone(&state.app_state)),
        )
        .route(
            "/verify_cert_valid/",
            post(verify_cert_valid).with_state(Arc::clone(&state.app_state)),
        )
        .route(
            "/verify_cert_bundle/",
            post(verify_cert_bundle).with_state(Arc::clone(&state.app_state)),
        )
        .route("/echo/", get(echo))
        .route("/hello/", get(hello))
        .route(
            "/nsm_desc",
            get(nsm_desc).with_state(Arc::clone(&state.app_state)),
        )
        .route(
            "/rng_seq",
            get(rng_seq).with_state(Arc::clone(&state.app_state)),
        )
        .with_state(state.clone());

    // Start HTTPS server
    let listening_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), ports.https);

    info!("Starting HTTPS server on {}", listening_address);

    axum_server::bind_openssl(listening_address, tls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .context("HTTPS server error")?;

    info!("Server shutdown complete");
    Ok(())
}
