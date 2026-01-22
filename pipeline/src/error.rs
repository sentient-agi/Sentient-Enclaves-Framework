use thiserror::Error;

/// Custom error types for the pipeline crate
#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("Socket error: {0}")]
    SocketError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Send error: failed to send {bytes} bytes - {message}")]
    SendError { bytes: usize, message: String },

    #[error("Receive error: failed to receive {bytes} bytes - {message}")]
    RecvError { bytes: usize, message: String },

    #[error("File error: {operation} failed for '{path}' - {message}")]
    FileError {
        operation: String,
        path: String,
        message: String,
    },

    #[error("Directory error: {operation} failed for '{path}' - {message}")]
    DirectoryError {
        operation: String,
        path: String,
        message: String,
    },

    #[error("Command execution error: {0}")]
    CommandError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Parse error: {field} - {message}")]
    ParseError { field: String, message: String },

    #[error("Argument error: {0}")]
    ArgumentError(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Invalid command ID: {0}")]
    InvalidCommandId(u64),

    #[error("Shutdown error: {0}")]
    ShutdownError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

/// Type alias for Results using PipelineError
pub type Result<T> = std::result::Result<T, PipelineError>;
