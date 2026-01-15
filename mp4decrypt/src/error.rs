use thiserror::Error;

/// The error type returned by decrypt operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Data too large (maximum supported {} bytes)", u32::MAX)]
    DataTooLarge,

    #[error("Failed to decrypt data with error code {0}")]
    DecryptionFailed(i32),

    #[error("Invalid hex format '{input}': {message}")]
    InvalidHex { input: String, message: String },

    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),

    #[error("No keys provided")]
    NoKeys,
}
