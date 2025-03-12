/// Remote attestation web-server for Sentient Enclaves Framework

use axum::{
    extract::{Query, State},
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri},
    response::{Redirect, Html},
    routing::get,
    routing::post,
    BoxError, Router,
};
use axum_extra::extract::Host;
use axum_server::tls_rustls::RustlsConfig;

use std::{future::Future, net::SocketAddr, time::Duration};
use tokio::signal;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use tracing_subscriber::fmt::format;
use tracing::{debug, info, error};

use aws_nitro_enclaves_nsm_api::api::{Digest as NsmDigest, Request, Response, AttestationDoc};
use aws_nitro_enclaves_nsm_api::driver::{nsm_exit, nsm_init, nsm_process_request};
use serde_bytes::ByteBuf;
use aws_nitro_enclaves_cose::CoseSign1;
use aws_nitro_enclaves_cose::crypto::openssl::Openssl;
use rand_core::{RngCore, OsRng}; // requires 'getrandom' feature

use std::{
    collections::{HashMap, BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
    fs::{self, DirEntry},
    path::PathBuf,
};
use std::io::Read;
use std::pin::Pin;

// use async_std::fs::{self, File};
// use async_std::io::{self, BufWriter, Read};
// use std::fs::{self, File};
// use std::io::{self, BufWriter, Read};
use async_std::io as async_io;
use async_std::fs as async_fs;
use async_std::path::Path;
use async_std::prelude::*;
use async_std::task::{Context, Poll, spawn_blocking};
use futures::task::noop_waker;
// use futures::FutureExt;
use futures::future::{BoxFuture, FutureExt};

use sha3::{Digest, Sha3_512};

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

type CachedState = Arc<RwLock<ServerState>>;

#[derive(Default)]
struct ServerState {
    nsm_fd: i32,
    docs: HashMap<String, AttData>,
}

#[derive(Default)]
struct AttData {
    filename: String,
    sha3_hash: String,
    vrf_hash: String,
    att_doc: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<(), String> {
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
    cached_state.write().unwrap().nsm_fd = fd;

    //Create a handle for our TLS server so the shutdown signal can all shutdown
    let handle = axum_server::Handle::new();
    //save the future for easy shutting down of redirect server
    let shutdown_future = shutdown_signal(handle.clone(), Arc::clone(&cached_state));
    // spawn a second server to redirect http requests to this server
    tokio::spawn(redirect_http_to_https(ports, shutdown_future));

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
        .route("/hello", get(hello))
        .route("/nsm_desc", get(nsm_desc))
        .route("/rng_seq", get(rng_seq))
        .route("/att_docs", get(att_docs))
        .route("/gen_att_docs", get(gen_att_docs))
        .with_state(Arc::clone(&cached_state));

    // run https server
    use std::str::FromStr;
    let listening_address = core::net::SocketAddr::new(
        core::net::IpAddr::V4(
            core::net::Ipv4Addr::from_str("127.0.0.1").unwrap()
        ),
        ports.https
    );
    debug!("listening on {listening_address}");
    axum_server::bind_rustls(listening_address, tls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn hello(Query(query_params): Query<HashMap<String, String>>, State(cached_state): State<CachedState>)
    -> Html<&'static str> {
        info!("{query_params:?}");
        let fd = cached_state.read().unwrap().nsm_fd;
        info!("fd: {fd:?}");

        match query_params.get("view").unwrap().as_str() {
            "bin" | "raw" => (),
            "hex" => (),
            "fmt" | "str" => (),
            _ => (),
        }

        Html("<h1>Hello, World!</h1>\n")
    }

async fn nsm_desc(Query(query_params): Query<HashMap<String, String>>, State(cached_state): State<CachedState>) -> String {
    info!("{query_params:?}");
    let fd = cached_state.read().unwrap().nsm_fd;
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
    let fd = cached_state.read().unwrap().nsm_fd;
    let randomness_sequence = get_randomness_sequence(fd);
    format!("{:?}\n", hex::encode(randomness_sequence))
}

async fn att_docs(Query(query_params): Query<HashMap<String, String>>, State(cached_state): State<CachedState>) -> String {
    info!("{query_params:?}");
    let fd = cached_state.read().unwrap().nsm_fd;

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

#[axum_macros::debug_handler]
async fn gen_att_docs(Query(query_params): Query<HashMap<String, String>>, State(cached_state): State<CachedState>) -> String {
    info!("{query_params:?}");
    let fd = cached_state.read().unwrap().nsm_fd;

    let path = match query_params.get("path").ok_or_else(|| "Missing path parameter in request".to_string()) {
        Ok(val) => val.to_owned(),
        Err(error) => {
            error!("No path in request query parameters or path is empty: {:?}", error);
            "".to_string().to_owned()
        }
    };

    if path.is_empty() {
        error!("Missing path parameter in request");
        return "Missing path parameter in request".to_string()
    };

    let fs_path = PathBuf::from(path.clone());

//    let results = recursive_hash_dir(path.as_str()).await.unwrap();
    let _results = recursive_hash_dir(path.as_str());

//    format!("\nFinal Hash Results:");
//    for (file_path, hash) in results {
//        format!("{}: {}", file_path, hex::encode(hash));
//    }

    "".to_string()
}

/// Recursively hashes all files in the given directory, using a `HashMap` for task tracking.
async fn recursive_hash_dir(dir_path: &str) -> async_io::Result<HashMap<String, Vec<u8>>> {
    let mut results = HashMap::new();
    let tasks: Arc<async_std::sync::Mutex<HashMap<String, Pin<Box<async_std::task::JoinHandle<std::io::Result<Vec<u8>>>>>>>> = Arc::new(async_std::sync::Mutex::new(HashMap::new()));

    visit_files_recursively(Path::new(dir_path), tasks.clone()).await?;

    // Check readiness of tasks
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    loop {
        let mut tasks_lock = tasks.lock().await;
        if tasks_lock.is_empty() {
            break;
        }

        let mut completed_tasks = Vec::new();

        for (file_path, task) in tasks_lock.iter_mut() {
            if check_task_readiness(file_path, task, &mut results, &mut cx).await {
                completed_tasks.push(file_path.clone());
            }
        }

        // Remove completed tasks from the hashmap
        for file_path in completed_tasks {
            tasks_lock.remove(&file_path);
        }

        // Yield to allow other async tasks to make progress
        async_std::task::yield_now().await;
    }

    Ok(results)
}

/// Checks the readiness of a specific task by file path.
async fn check_task_readiness(
    file_path: &str,
    task: &mut Pin<Box<async_std::task::JoinHandle<std::io::Result<Vec<u8>>>>>,
    results: &mut HashMap<String, Vec<u8>>,
    cx: &mut Context<'_>,
) -> bool {
    match task.as_mut().poll(cx) {
        Poll::Ready(Ok(hash)) => {
            results.insert(file_path.to_string(), hash);
            true // Task is complete
        }
        Poll::Ready(Err(e)) => {
            eprintln!("Error processing {}: {}", file_path, e);
            true // Task is complete
        }
//        Poll::Ready(Err(e)) => {
//            eprintln!("Task panicked for {}: {}", file_path, e);
//            true // Task is complete
//        }
        Poll::Pending => false, // Task is not complete
    }
}

/// Visits all files recursively in a directory and spawns hashing tasks.
fn visit_files_recursively<'a>(
    path: &'a Path,
    tasks: Arc<async_std::sync::Mutex<HashMap<String, Pin<Box<async_std::task::JoinHandle<std::io::Result<Vec<u8>>>>>>>>,
) -> BoxFuture<'a, async_io::Result<()>> {
    async move {
        if path.is_dir().await {
            let mut entries = async_fs::read_dir(path).await?;
            while let Some(entry) = entries.next().await {
                let entry = entry?;
                visit_files_recursively(entry.path().as_path(), tasks.clone()).await?;
            }
        } else if path.is_file().await {
            let file_path_hash = path.to_string_lossy().to_string();
            let file_path_task = file_path_hash.clone();
            let task = spawn_blocking(move || {
                // Perform hashing in a separate thread
                hash_file(&file_path_hash)
            });
            tasks.lock().await.insert(file_path_task, Box::pin(task));
        }
        Ok(())
    }.boxed()
}

/// Hashes a single file using SHA3-512.
fn hash_file(file_path: &str) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(file_path)?;
    let mut hasher = Sha3_512::new();
    let mut buffer = [0; 8192];

    // Read the file in chunks and update the hash
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    // Finalize and return the hash
    Ok(hasher.finalize().to_vec())
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
    let fd = cached_state.read().unwrap().nsm_fd;
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
    digest: NsmDigest,
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
