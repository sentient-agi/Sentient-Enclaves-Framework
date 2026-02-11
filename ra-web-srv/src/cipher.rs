//! Cipher suite mapping and utilities

use openssl::nid::Nid as CipherID;
use vrf::openssl::CipherSuite;

/// Trait for mapping cipher suites between different representations
pub trait CipherMapper {
    /// Convert CipherSuite to openssl::nid::Nid
    fn to_nid(&self) -> CipherID;
    /// Convert CipherSuite type to corresponding string
    fn to_string(&self) -> String;
    /// Convert string with cipher suite to the corresponding CipherSuite type
    fn from_string(suite_string: &str) -> Result<CipherSuite, String>;
}

impl CipherMapper for CipherSuite {
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
            _ => return Err(format!("Unknown cipher suite: {}", suite_string)),
        };
        Ok(cipher_suite)
    }
}
