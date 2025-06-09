/// The returned error type.
#[derive(Debug)]
pub struct Error {
    pub msg: String,
    pub err_type: ErrorType,
}

/// The type of error which can occur during decryption.
#[derive(Debug)]
pub enum ErrorType {
    DataTooLarge,
    Failed(i32),
    InvalidFormat,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mp4decrypt-error: {}", self.msg)
    }
}

impl std::error::Error for Error {}
