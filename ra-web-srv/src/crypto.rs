//! Cryptographic operations including key generation and VRF

use crate::cipher::CipherMapper;
use crate::errors::{AppResult, CryptoError, VrfError};
use openssl::nid::Nid as CipherID;
use openssl::pkey::{PKey, Private, Public};
use tracing::{debug, error, info};
use vrf::openssl::{CipherSuite, ECVRF};
use vrf::VRF;

/// Generate an EC keypair for the given cipher ID
pub fn generate_keypair(cipher_id: CipherID) -> AppResult<(PKey<Private>, PKey<Public>)> {
    debug!("Generating EC keypair for cipher: {:?}", cipher_id);

    let alg = openssl::ec::EcGroup::from_curve_name(cipher_id).map_err(|e| {
        error!("Failed to create EC group for cipher {:?}: {}", cipher_id, e);
        CryptoError::KeyGenError(format!("Failed to create EC group: {}", e))
    })?;

    let ec_private = openssl::ec::EcKey::generate(&alg).map_err(|e| {
        error!("Failed to generate EC private key: {}", e);
        CryptoError::KeyGenError(format!("Failed to generate EC key: {}", e))
    })?;

    let ec_public = openssl::ec::EcKey::from_public_key(&alg, ec_private.public_key()).map_err(|e| {
        error!("Failed to derive EC public key: {}", e);
        CryptoError::KeyGenError(format!("Failed to derive public key: {}", e))
    })?;

    let private_pkey = PKey::from_ec_key(ec_private).map_err(|e| {
        error!("Failed to convert EC private key to PKey: {}", e);
        CryptoError::KeyConversionError(format!("Failed to convert private key: {}", e))
    })?;

    let public_pkey = PKey::from_ec_key(ec_public).map_err(|e| {
        error!("Failed to convert EC public key to PKey: {}", e);
        CryptoError::KeyConversionError(format!("Failed to convert public key: {}", e))
    })?;

    debug!("EC keypair generated successfully");
    Ok((private_pkey, public_pkey))
}

/// Generate PRIME256V1/P-256 EC keypair
#[allow(dead_code)]
pub fn generate_ec256_keypair() -> AppResult<(PKey<Private>, PKey<Public>)> {
    generate_keypair(CipherID::X9_62_PRIME256V1)
}

/// Generate SECP384R1/P-384 EC keypair
#[allow(dead_code)]
pub fn generate_ec384_keypair() -> AppResult<(PKey<Private>, PKey<Public>)> {
    generate_keypair(CipherID::SECP384R1)
}

/// Generate SECP521R1/P-521 EC keypair
pub fn generate_ec512_keypair() -> AppResult<(PKey<Private>, PKey<Public>)> {
    generate_keypair(CipherID::SECP521R1)
}

/// Generate a VRF proof for the given message
pub fn vrf_proof(message: &[u8], secret_key: &[u8], cipher_suite: CipherSuite) -> AppResult<Vec<u8>> {
    debug!("Generating VRF proof for message of {} bytes", message.len());

    let mut vrf = ECVRF::from_suite(cipher_suite).map_err(|e| {
        error!("Failed to create VRF from cipher suite: {:?}", e);
        VrfError::SuiteCreationError(format!("{:?}", e))
    })?;

    let _public_key = vrf.derive_public_key(secret_key).map_err(|e| {
        error!("Failed to derive VRF public key: {:?}", e);
        VrfError::PublicKeyDerivationError(format!("{:?}", e))
    })?;

    let proof = vrf.prove(secret_key, message).map_err(|e| {
        error!("Failed to generate VRF proof: {:?}", e);
        VrfError::ProofGenerationError(format!("{:?}", e))
    })?;

    debug!("VRF proof generated: {} bytes", proof.len());
    Ok(proof)
}

/// Verify a VRF proof
pub fn vrf_verify(
    message: &[u8],
    proof: &[u8],
    public_key: &[u8],
    cipher_suite: CipherSuite,
) -> Result<String, String> {
    debug!("Verifying VRF proof");

    let mut vrf = ECVRF::from_suite(cipher_suite).map_err(|e| {
        let msg = format!("Failed to create VRF from cipher suite: {:?}", e);
        error!("{}", msg);
        msg
    })?;

    let hash = vrf.proof_to_hash(proof).map_err(|e| {
        let msg = format!("Failed to convert proof to hash: {:?}", e);
        error!("{}", msg);
        msg
    })?;

    match vrf.verify(public_key, proof, message) {
        Ok(outcome) => {
            if hash == outcome {
                info!("VRF proof is valid");
                Ok("VRF proof is valid!".to_string())
            } else {
                error!("VRF proof hash mismatch");
                Err("VRF proof is not valid: hash mismatch".to_string())
            }
        }
        Err(e) => {
            let msg = format!("VRF proof verification failed: {:?}", e);
            error!("{}", msg);
            Err(msg)
        }
    }
}

/// Extract public key from private key bytes in PEM format
pub fn extract_public_key_from_pem(
    private_key_pem: &[u8],
    cipher_id: CipherID,
) -> AppResult<Vec<u8>> {
    let pkey = PKey::private_key_from_pem(private_key_pem).map_err(|e| {
        error!("Failed to parse private key from PEM: {}", e);
        CryptoError::InvalidKeyFormat(format!("Failed to parse PEM: {}", e))
    })?;

    let ec_key = pkey.ec_key().map_err(|e| {
        error!("Failed to get EC key from PKey: {}", e);
        CryptoError::EcKeyError(format!("Failed to get EC key: {}", e))
    })?;

    let alg = openssl::ec::EcGroup::from_curve_name(cipher_id).map_err(|e| {
        error!("Failed to create EC group: {}", e);
        CryptoError::KeyGenError(format!("Failed to create EC group: {}", e))
    })?;

    let ec_pubkey = openssl::ec::EcKey::from_public_key(&alg, ec_key.public_key()).map_err(|e| {
        error!("Failed to create public EC key: {}", e);
        CryptoError::KeyConversionError(format!("Failed to create public key: {}", e))
    })?;

    let pkey_pubkey = PKey::from_ec_key(ec_pubkey).map_err(|e| {
        error!("Failed to convert EC public key to PKey: {}", e);
        CryptoError::KeyConversionError(format!("Failed to convert public key: {}", e))
    })?;

    let pubkey_pem = pkey_pubkey.public_key_to_pem().map_err(|e| {
        error!("Failed to convert public key to PEM: {}", e);
        CryptoError::PemError(format!("Failed to convert to PEM: {}", e))
    })?;

    Ok(pubkey_pem)
}
