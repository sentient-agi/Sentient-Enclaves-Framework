//! Custom error types for the Remote Attestation Web Server

use thiserror::Error;

/// Main application error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("NSM error: {0}")]
    NsmError(String),

    #[error("Cryptographic error: {0}")]
    CryptoError(String),

    #[error("VRF error: {0}")]
    VrfError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("NATS error: {0}")]
    NatsError(String),

    #[error("Attestation error: {0}")]
    AttestationError(String),

    #[error("Certificate error: {0}")]
    CertificateError(String),

    #[error("TLS error: {0}")]
    TlsError(String),

    #[error("Hex decode error: {0}")]
    HexError(#[from] hex::FromHexError),

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("File system error: {0}")]
    FsError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Configuration-specific errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file '{path}': {source}")]
    ReadError {
        path: String,
        source: std::io::Error,
    },

    #[error("Failed to parse config file '{path}': {message}")]
    ParseError {
        path: String,
        message: String,
    },

    #[error("Failed to serialize config: {0}")]
    SerializeError(String),

    #[error("Failed to write config file '{path}': {source}")]
    WriteError {
        path: String,
        source: std::io::Error,
    },

    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),
}

/// NSM (Nitro Security Module) errors
#[derive(Error, Debug)]
pub enum NsmError {
    #[error("NSM initialization failed with code: {0}")]
    InitError(i32),

    #[error("Invalid NSM file descriptor: {0}")]
    InvalidFd(i32),

    #[error("NSM request failed: {0}")]
    RequestError(String),

    #[error("Invalid NSM response: {0}")]
    InvalidResponse(String),

    #[error("NSM description error: {0}")]
    DescriptionError(String),

    #[error("Empty attestation document")]
    EmptyDocument,

    #[error("Empty random sequence")]
    EmptyRandom,

    #[error("Random sequence mismatch")]
    RandomMismatch,
}

/// Cryptographic operation errors
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Key generation failed: {0}")]
    KeyGenError(String),

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),

    #[error("Key conversion failed: {0}")]
    KeyConversionError(String),

    #[error("Signature verification failed: {0}")]
    SignatureError(String),

    #[error("OpenSSL error: {0}")]
    OpenSslError(String),

    #[error("Invalid cipher suite: {0}")]
    InvalidCipherSuite(String),

    #[error("EC key error: {0}")]
    EcKeyError(String),

    #[error("PEM conversion error: {0}")]
    PemError(String),

    #[error("DER conversion error: {0}")]
    DerError(String),
}

/// VRF (Verifiable Random Function) errors
#[derive(Error, Debug)]
pub enum VrfError {
    #[error("VRF suite creation failed: {0}")]
    SuiteCreationError(String),

    #[error("VRF proof generation failed: {0}")]
    ProofGenerationError(String),

    #[error("VRF verification failed: {0}")]
    VerificationError(String),

    #[error("Public key derivation failed: {0}")]
    PublicKeyDerivationError(String),

    #[error("Nonce generation failed: {0}")]
    NonceGenerationError(String),

    #[error("Proof to hash conversion failed: {0}")]
    ProofToHashError(String),
}

/// Attestation-related errors
#[derive(Error, Debug)]
pub enum AttestationError {
    #[error("COSE document parsing failed: {0}")]
    CoseParseError(String),

    #[error("Attestation document parsing failed: {0}")]
    AttDocParseError(String),

    #[error("Invalid attestation document: {0}")]
    InvalidAttDoc(String),

    #[error("Certificate parsing failed: {0}")]
    CertParseError(String),

    #[error("Certificate validation failed: {0}")]
    CertValidationError(String),

    #[error("Certificate expired")]
    CertExpired,

    #[error("Certificate not yet valid")]
    CertNotYetValid,

    #[error("CA bundle empty")]
    EmptyCaBundle,

    #[error("Signature verification failed: {0}")]
    SignatureVerificationError(String),
}

/// NATS messaging errors
#[derive(Error, Debug)]
pub enum NatsError {
    #[error("NATS connection failed: {0}")]
    ConnectionError(String),

    #[error("NATS KV bucket error: {0}")]
    KvBucketError(String),

    #[error("NATS key not found: {0}")]
    KeyNotFound(String),

    #[error("NATS put operation failed: {0}")]
    PutError(String),

    #[error("NATS get operation failed: {0}")]
    GetError(String),

    #[error("NATS watch error: {0}")]
    WatchError(String),

    #[error("NATS flush error: {0}")]
    FlushError(String),
}

/// HTTP handler errors
#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    #[error("Invalid parameter value: {0}")]
    InvalidParameter(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Processing in progress")]
    Processing,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl From<ConfigError> for AppError {
    fn from(e: ConfigError) -> Self {
        AppError::ConfigError(e.to_string())
    }
}

impl From<NsmError> for AppError {
    fn from(e: NsmError) -> Self {
        AppError::NsmError(e.to_string())
    }
}

impl From<CryptoError> for AppError {
    fn from(e: CryptoError) -> Self {
        AppError::CryptoError(e.to_string())
    }
}

impl From<VrfError> for AppError {
    fn from(e: VrfError) -> Self {
        AppError::VrfError(e.to_string())
    }
}

impl From<AttestationError> for AppError {
    fn from(e: AttestationError) -> Self {
        AppError::AttestationError(e.to_string())
    }
}

impl From<NatsError> for AppError {
    fn from(e: NatsError) -> Self {
        AppError::NatsError(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::SerializationError(e.to_string())
    }
}

impl From<toml::de::Error> for AppError {
    fn from(e: toml::de::Error) -> Self {
        AppError::ConfigError(format!("TOML parse error: {}", e))
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(e: toml::ser::Error) -> Self {
        AppError::SerializationError(format!("TOML serialize error: {}", e))
    }
}

impl From<openssl::error::ErrorStack> for AppError {
    fn from(e: openssl::error::ErrorStack) -> Self {
        AppError::CryptoError(format!("OpenSSL error: {}", e))
    }
}

impl From<openssl::error::ErrorStack> for CryptoError {
    fn from(e: openssl::error::ErrorStack) -> Self {
        CryptoError::OpenSslError(e.to_string())
    }
}

impl From<vrf::openssl::Error> for VrfError {
    fn from(e: vrf::openssl::Error) -> Self {
        VrfError::VerificationError(format!("{:?}", e))
    }
}

impl From<aws_nitro_enclaves_cose::error::CoseError> for AttestationError {
    fn from(e: aws_nitro_enclaves_cose::error::CoseError) -> Self {
        AttestationError::CoseParseError(format!("{:?}", e))
    }
}

/// Result type alias for application operations
pub type AppResult<T> = anyhow::Result<T>;

/// Result type alias for handler operations
pub type HandlerResult<T> = Result<T, HandlerError>;
