//! Request types for HTTP handlers

use serde::Deserialize;

/// Request for generating attestation documents
#[derive(Default, Debug, Clone, Deserialize)]
pub struct GenerateRequest {
    pub path: String,
}

/// Request for verifying PCR registers
#[derive(Default, Debug, Clone, Deserialize)]
pub struct VerifyPCRsRequest {
    pub pcrs: String,
}

/// Request for verifying file hash
#[derive(Default, Debug, Clone, Deserialize)]
pub struct VerifyHashRequest {
    pub file_path: String,
    pub sha3_hash: String,
}

/// Request for verifying VRF proof
#[derive(Default, Debug, Clone, Deserialize)]
pub struct VerifyProofRequest {
    pub user_data: String,
    pub public_key: String,
}

/// Request for verifying attestation document
#[derive(Default, Debug, Clone, Deserialize)]
pub struct VerifyDocRequest {
    pub cose_doc_bytes: String,
}
