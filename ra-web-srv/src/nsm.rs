//! NSM (Nitro Security Module) device operations

use crate::errors::{AppResult, NsmError};
use aws_nitro_enclaves_cose::error::CoseError;
use aws_nitro_enclaves_nsm_api::api::{Digest as NsmDigest, Request, Response};
use aws_nitro_enclaves_nsm_api::driver::nsm_process_request;
use serde_bytes::ByteBuf;
use std::collections::BTreeSet;
use std::str::FromStr;
use tracing::{debug, error, info};

/// NSM device description
pub struct NsmDescription {
    pub version_major: u16,
    pub version_minor: u16,
    pub version_patch: u16,
    pub module_id: String,
    pub max_pcrs: u16,
    pub locked_pcrs: BTreeSet<u16>,
    pub digest: NsmDigest,
}

/// Local wrapper for NsmDigest with Display implementation
#[derive(Debug, Clone)]
pub struct LocalNsmDigest(pub NsmDigest);

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

/// Get NSM device description
pub fn get_nsm_description(fd: i32) -> AppResult<NsmDescription> {
    debug!("Requesting NSM description for fd: {}", fd);

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
        } => {
            info!(
                "NSM description retrieved: module_id={}, version={}.{}.{}, max_pcrs={}",
                module_id, version_major, version_minor, version_patch, max_pcrs
            );
            Ok(NsmDescription {
                version_major,
                version_minor,
                version_patch,
                module_id,
                max_pcrs,
                locked_pcrs,
                digest,
            })
        }
        other => {
            let msg = format!("Request::DescribeNSM got invalid response: {:?}", other);
            error!("{}", msg);
            Err(NsmError::InvalidResponse(msg).into())
        }
    }
}

/// Get a random byte sequence from NSM
pub fn get_randomness_sequence(fd: i32, count_bytes: usize) -> AppResult<Vec<u8>> {
    debug!("Requesting {} random bytes from NSM", count_bytes);

    let mut prev_random: Vec<u8> = vec![];
    let mut random_bytes: Vec<u8> = vec![];
    let random_gen_cycles = 128;

    while random_bytes.len() < count_bytes {
        for _ in 0..random_gen_cycles {
            let random = match nsm_process_request(fd, Request::GetRandom) {
                Response::GetRandom { random } => {
                    if random.is_empty() {
                        error!("NSM returned empty random sequence");
                        return Err(NsmError::EmptyRandom.into());
                    }
                    if prev_random == random {
                        error!("NSM returned duplicate random sequence");
                        return Err(NsmError::RandomMismatch.into());
                    }
                    prev_random = random.clone();
                    debug!("Received {} random bytes", random.len());
                    random
                }
                resp => {
                    let msg = format!(
                        "GetRandom: expecting Response::GetRandom, but got {:?}",
                        resp
                    );
                    error!("{}", msg);
                    return Err(NsmError::InvalidResponse(msg).into());
                }
            };
            random_bytes.extend(random);
        }
    }

    let result = random_bytes[..count_bytes].to_vec();
    debug!("Generated {} random bytes", result.len());
    Ok(result)
}

/// Get an attestation document from NSM
pub fn get_attestation_doc(
    fd: i32,
    user_data: Option<ByteBuf>,
    nonce: Option<ByteBuf>,
    public_key: Option<ByteBuf>,
) -> AppResult<Vec<u8>> {
    debug!(
        "Requesting attestation document (user_data: {}, nonce: {}, public_key: {})",
        user_data.is_some(),
        nonce.is_some(),
        public_key.is_some()
    );

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
            if document.is_empty() {
                error!("NSM returned empty COSE document");
                return Err(NsmError::EmptyDocument.into());
            }
            info!("COSE document received: {} bytes", document.len());
            Ok(document)
        }
        other => {
            let msg = format!("Request::Attestation got invalid response: {:?}", other);
            error!("{}", msg);
            Err(NsmError::InvalidResponse(msg).into())
        }
    }
}
