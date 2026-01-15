use thiserror::Error;

/// Error types returned by the mp4decrypt operations.
///
/// This enum represents all possible errors that can occur during
/// processor creation and decryption operations.
#[derive(Debug, Error)]
pub enum Error {
    /// The input data exceeds the maximum supported size (4GB).
    ///
    /// This occurs when the combined size of the init segment and media segment
    /// exceeds `u32::MAX` bytes.
    #[error("Data too large (maximum supported {} bytes)", u32::MAX)]
    DataTooLarge,

    /// Decryption failed with an error code from the Bento4 library.
    ///
    /// Common causes include:
    /// - Incorrect decryption key for the given content
    /// - Corrupted or invalid MP4 data
    /// - Missing or mismatched initialization segment
    #[error("Failed to decrypt data with error code {0}")]
    DecryptionFailed(i32),

    /// The provided hex string is valid but has an incorrect length.
    ///
    /// Both KID and key must be exactly 32 hexadecimal characters (16 bytes).
    #[error("Invalid hex format '{input}': {message}")]
    InvalidHex {
        /// The invalid input string
        input: String,
        /// Description of why the hex is invalid
        message: String,
    },

    /// The provided string is not valid hexadecimal.
    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),

    /// No decryption keys were provided to the builder.
    ///
    /// At least one KID/key pair must be added using
    /// [`Ap4CencDecryptingProcessorBuilder::key`](crate::Ap4CencDecryptingProcessorBuilder::key) or
    /// [`Ap4CencDecryptingProcessorBuilder::keys`](crate::Ap4CencDecryptingProcessorBuilder::keys)
    /// before calling [`build`](crate::Ap4CencDecryptingProcessorBuilder::build).
    #[error("No keys provided")]
    NoKeys,
}
