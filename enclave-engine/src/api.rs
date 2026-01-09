use crate::config::EnclaveConfig;
use crate::service::{EnclaveService, EnclaveInstance};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

pub fn create_router(service: EnclaveService) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/enclaves", post(provision_enclave))
        .route("/enclaves", get(list_enclaves))
        .route("/enclaves/:name", get(get_enclave_status))
        .route("/enclaves/:name", delete(delete_enclave))
        .route("/enclaves/:name/stop", post(stop_enclave))
        .route("/system/numa", get(get_numa_info))
        .route("/system/hugepages", get(get_hugepages_info))
        .with_state(Arc::new(service))
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "enclave-engine"
    }))
}

async fn provision_enclave(
    State(service): State<Arc<EnclaveService>>,
    Json(config): Json<EnclaveConfig>,
) -> Result<Json<ProvisionResponse>, ApiError> {
    let id = service.provision(config).await?;
    
    Ok(Json(ProvisionResponse {
        id,
        message: "Enclave provisioned successfully".to_string(),
    }))
}

async fn list_enclaves(
    State(service): State<Arc<EnclaveService>>,
) -> Result<Json<Vec<EnclaveInstance>>, ApiError> {
    let enclaves = service.list().await?;
    Ok(Json(enclaves))
}

async fn get_enclave_status(
    State(service): State<Arc<EnclaveService>>,
    Path(name): Path<String>,
) -> Result<Json<EnclaveInstance>, ApiError> {
    let instance = service.status(&name).await?;
    Ok(Json(instance))
}

async fn stop_enclave(
    State(service): State<Arc<EnclaveService>>,
    Path(name): Path<String>,
) -> Result<Json<MessageResponse>, ApiError> {
    service.stop(&name).await?;
    
    Ok(Json(MessageResponse {
        message: format!("Enclave {} stopped", name),
    }))
}

async fn delete_enclave(
    State(service): State<Arc<EnclaveService>>,
    Path(name): Path<String>,
) -> Result<Json<MessageResponse>, ApiError> {
    service.delete(&name).await?;
    
    Ok(Json(MessageResponse {
        message: format!("Enclave {} deleted", name),
    }))
}

async fn get_numa_info(
    State(service): State<Arc<EnclaveService>>,
) -> Result<Json<SystemInfoResponse>, ApiError> {
    let info = service.get_numa_info().await?;
    
    Ok(Json(SystemInfoResponse { info }))
}

async fn get_hugepages_info(
    State(service): State<Arc<EnclaveService>>,
) -> Result<Json<SystemInfoResponse>, ApiError> {
    let info = service.get_hugepages_info().await?;
    
    Ok(Json(SystemInfoResponse { info }))
}

#[derive(Debug, Serialize)]
struct ProvisionResponse {
    id: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Debug, Serialize)]
struct SystemInfoResponse {
    info: String,
}

#[derive(Debug)]
struct ApiError(crate::error::EnclaveError);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            crate::error::EnclaveError::NotFound(ref msg) => {
                (StatusCode::NOT_FOUND, msg.clone())
            }
            crate::error::EnclaveError::AlreadyExists(ref msg) => {
                (StatusCode::CONFLICT, msg.clone())
            }
            crate::error::EnclaveError::Config(ref msg) => {
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            _ => {
                let msg = self.0.to_string();
                error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };
        
        let body = Json(serde_json::json!({
            "error": message
        }));
        
        (status, body).into_response()
    }
}

impl<E> From<E> for ApiError
where
    E: Into<crate::error::EnclaveError>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}