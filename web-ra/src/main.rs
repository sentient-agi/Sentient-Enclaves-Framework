/// Remote attestation web-server for Sentient Secure Enclaves toolkit (aka Sentinel)

use axum::{
    extract::{Query, State},
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri},
    response::{Redirect, Html},
    routing::get,
    BoxError, Router,
};
use axum_extra::extract::Host;
use axum_server::tls_rustls::RustlsConfig;
use std::{future::Future, net::SocketAddr, path::PathBuf, time::Duration};
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use tracing_subscriber::fmt::format;
use tracing::{debug, info, error};

use aws_nitro_enclaves_nsm_api::api::{Digest, Request, Response, AttestationDoc};
use aws_nitro_enclaves_nsm_api::driver::{nsm_exit, nsm_init, nsm_process_request};
use serde_bytes::ByteBuf;
use std::collections::BTreeSet;
use aws_nitro_enclaves_cose::CoseSign1;
use aws_nitro_enclaves_cose::crypto::openssl::Openssl;
use rand_core::{RngCore, OsRng}; // requires 'getrandom' feature

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

type CachedState = Arc<RwLock<ServerState>>;

#[derive(Default)]
struct ServerState {
    fds: HashMap<String, i32>,
    att_docs: HashMap<String, Vec<u8>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let ports = Ports {
        http: 8080,
        https: 8443,
    };

    // let fd = 3;
    let fd = nsm_init();
    assert!(fd >= 0, "[Error] NSM initialization returned {}.", fd);
    info!("NSM device initialized.");

    let cached_state = CachedState::default();
    cached_state.write().unwrap().fds.insert("default_fd".to_string(), fd);

    //Create a handle for our TLS server so the shutdown signal can all shutdown
    let handle = axum_server::Handle::new();
    //save the future for easy shutting down of redirect server
    let shutdown_future = shutdown_signal(handle.clone(), Arc::clone(&cached_state));

    // optional: spawn a second server to redirect http requests to this server
    tokio::spawn(redirect_http_to_https(ports, shutdown_future));
/*
    // configure certificate and private key used by https
    let config = RustlsConfig::from_pem_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("cert.pem"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self_signed_certs")
            .join("key.pem"),
    )
    .await
    .unwrap();
*/
    // configure certificate and private key used by https
    let config = RustlsConfig::from_pem_file(
        PathBuf::from("/app")
            .join("self_signed_certs")
            .join("cert.pem"),
        PathBuf::from("/app")
            .join("self_signed_certs")
            .join("key.pem"),
    )
    .await
    .unwrap();

    let app = Router::new()
        .route("/hello", get(hello).with_state(Arc::clone(&cached_state)))
        .route("/nsm_desc", get(nsm_desc).with_state(Arc::clone(&cached_state)))
        .route("/rng_seq", get(rng_seq).with_state(Arc::clone(&cached_state)))
        .route("/att_doc", get(att_doc).with_state(Arc::clone(&cached_state)));

    // run https server
    let addr = SocketAddr::from(([127, 0, 0, 1], ports.https));
    debug!("listening on {addr}");
    axum_server::bind_rustls(addr, config)
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn hello(Query(query_params): Query<HashMap<String, String>>, State(cached_state): State<CachedState>)
    -> Html<&'static str> {
        info!("{query_params:?}");
        match query_params.get("view").unwrap().as_str() {
            "bin" | "raw" => (),
            "hex" => (),
            "fmt" | "str" => (),
            _ => (),
        }
        let fd = cached_state.read().unwrap().fds.get("default_fd").unwrap().clone();
        info!("fd: {fd:?}");
        Html("<h1>Hello, World!</h1>\n")
    }

async fn nsm_desc(Query(query_params): Query<HashMap<String, String>>, State(cached_state): State<CachedState>) -> String {
    info!("{query_params:?}");
    let fd = cached_state.read().unwrap().fds.get("default_fd").unwrap().clone();
    let description = get_nsm_description(fd);
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

async fn rng_seq(Query(query_params): Query<HashMap<String, String>>, State(cached_state): State<CachedState>) -> String {
    info!("{query_params:?}");
    let fd = cached_state.read().unwrap().fds.get("default_fd").unwrap().clone();
    let randomness_sequence = get_randomness_sequence(fd);
    format!("{:?}\n", hex::encode(randomness_sequence))
}

async fn att_doc(Query(query_params): Query<HashMap<String, String>>, State(cached_state): State<CachedState>) -> String {
    info!("{query_params:?}");
    let fd = cached_state.read().unwrap().fds.get("default_fd").unwrap().clone();
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

async fn shutdown_signal(handle: axum_server::Handle, cached_state: CachedState) {
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
    let fd = cached_state.read().unwrap().fds.get("default_fd").unwrap().clone();
    // close device file descriptor before app exit
    nsm_exit(fd);
    println!("NSM device closed.");
    handle.graceful_shutdown(Some(Duration::from_secs(10))); // 10 secs is how long docker will wait
                                                                  // to force shutdown
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

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
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

struct NsmDescription {
    version_major: u16,
    version_minor: u16,
    version_patch: u16,
    module_id: String,
    max_pcrs: u16,
    locked_pcrs: BTreeSet<u16>,
    digest: Digest,
}

fn get_nsm_description(fd: i32) -> NsmDescription {
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
        } => NsmDescription {
            version_major,
            version_minor,
            version_patch,
            module_id,
            max_pcrs,
            locked_pcrs,
            digest,
        },
        _ => {
            panic!(
                "[Error] Request::DescribeNSM got invalid response: {:?}",
                response
            )
        }
    }
}

fn get_randomness_sequence(fd: i32) -> Vec<u8> {
    let mut prev_random: Vec<u8> = vec![];
    let mut random: Vec<u8> = vec![];

    for _ in 0..16 {
        random = match nsm_process_request(fd, Request::GetRandom) {
            Response::GetRandom { random } => {
                assert!(!random.is_empty());
                assert!(prev_random != random);
                prev_random = random.clone();
                println!("Random bytes: {:?}", random.clone());
                random
            }
            resp => {
                println!(
                    "GetRandom: expecting Response::GetRandom, but got {:?} instead",
                    resp
                );
                vec![0u8; 64]
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
            println!("COSE document length: {:?} bytes", document.len());
            // println!("Attestation document: {:?}", document);
            document
        }
        _ => {
            println!(
                "[Error] Request::Attestation got invalid response: {:?}",
                response
            );
            vec![0u8, 64]
        },
    }
}
