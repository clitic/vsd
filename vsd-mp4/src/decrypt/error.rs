//! Error types for CENC/CBCS decryption operations.

use thiserror::Error;

/// Errors that can occur during CENC/CBCS decryption.
#[derive(Debug, Error)]
pub enum DecryptError {
    /// Invalid key size (must be 16 bytes for AES-128).
    #[error("invalid key size: expected 16 bytes for AES-128, got {0} bytes")]
    InvalidKeySize(usize),

    /// Invalid IV size.
    #[error("invalid IV size: expected {expected} bytes, got {actual} bytes")]
    InvalidIvSize { expected: usize, actual: usize },

    /// Invalid hex string.
    #[error("invalid hex string: {0}")]
    InvalidHex(#[from] hex::FromHexError),

    /// Hex string has wrong length.
    #[error("hex string has wrong length: expected 16 bytes (32 hex chars), got {0} bytes")]
    HexWrongLength(usize),

    /// Invalid MP4 format.
    #[error("invalid MP4 format: {0}")]
    InvalidFormat(String),

    /// Unsupported protection scheme.
    #[error("unsupported protection scheme: {0} (supported: cenc, cens, cbc1, cbcs)")]
    UnsupportedScheme(String),

    /// No decryption keys provided.
    #[error("no decryption keys provided - use .key(kid, key) to add keys")]
    NoKeys,

    /// Key not found for the given KID.
    #[error("key not found for KID: {0} - ensure the correct KID/key pair is provided")]
    KeyNotFound(String),

    /// Sample index out of range.
    #[error(
        "sample index out of range: requested index {index} but only {count} samples available"
    )]
    SampleIndexOutOfRange { index: usize, count: usize },

    /// Subsample data error.
    #[error("subsample data error: {0}")]
    SubsampleError(String),

    /// Cipher not initialized.
    #[error("cipher not initialized: ensure key and IV are set before decryption")]
    CipherNotInitialized,

    /// I/O error (for file operations).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for decryption operations.
pub type Result<T> = std::result::Result<T, DecryptError>;
