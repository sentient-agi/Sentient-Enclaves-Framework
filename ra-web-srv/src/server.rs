//! Server startup and shutdown logic

use crate::config::Ports;
use crate::state::AppState;
use aws_nitro_enclaves_nsm_api::driver::nsm_exit;
use axum::http::{StatusCode, Uri};
use axum::response::Redirect;
use axum::handler::HandlerWithoutStateExt;
use axum::BoxError;
use axum_extra::extract::Host;
use parking_lot::RwLock;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{debug, error, info};

/// Redirect HTTP to HTTPS
pub async fn redirect_http_to_https<F>(ports: Ports, signal: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();
        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse()?);
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
                error!("Failed to convert URI to HTTPS: {}", error);
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], ports.http));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind HTTP redirect listener on {}: {}", addr, e);
            return;
        }
    };

    debug!("HTTP redirect listening on {}", addr);

    if let Err(e) = axum::serve(listener, redirect.into_make_service())
        .with_graceful_shutdown(signal)
        .await
    {
        error!("HTTP redirect server error: {}", e);
    }
}

/// Handle shutdown signal
pub async fn shutdown_signal(handle: axum_server::Handle, app_state: Arc<RwLock<AppState>>) {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to install Ctrl+C handler: {}", e);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                error!("Failed to install SIGTERM handler: {}", e);
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Received termination signal, shutting down...");

    let fd = app_state.read().nsm_fd;
    nsm_exit(fd);
    info!("NSM device closed");

    handle.graceful_shutdown(Some(Duration::from_secs(10)));
}
