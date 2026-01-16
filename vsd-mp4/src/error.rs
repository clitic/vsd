use thiserror::Error;

/// The returned error type.
#[derive(Debug, Error)]
pub enum Error {
    #[error("vsd-mp4-error: {0}")]
    Generic(String),

    #[error("vsd-mp4-error: cannot decode {0}")]
    Decode(String),

    #[error("Reader failed to read data: {0}")]
    Read(#[from] std::io::Error),
}

impl Error {
    /// Create a new generic error.
    pub fn new<T: Into<String>>(msg: T) -> Self {
        Self::Generic(msg.into())
    }

    /// Create a new decode error.
    pub fn new_decode<T: Into<String>>(msg: T) -> Self {
        Self::Decode(msg.into())
    }
}
