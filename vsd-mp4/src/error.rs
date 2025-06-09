/// The returned error type.
#[derive(Debug)]
pub struct Error {
    pub msg: String,
    pub err_type: ErrorType,
}

/// The type of error which can occur during parsing data.
#[derive(Debug)]
pub enum ErrorType {
    Decode,
    Generic,
    Read,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "vsd-mp4-error: {}", self.msg)
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Create a new generic error.
    pub fn new<T: Into<String>>(msg: T) -> Self {
        Self {
            err_type: ErrorType::Generic,
            msg: msg.into(),
        }
    }

    /// Create a new decode error.
    pub fn new_decode<T: Into<String>>(msg: T) -> Self {
        Self {
            err_type: ErrorType::Decode,
            msg: format!("cannot decode {}", msg.into()),
        }
    }

    /// Create a new read error.
    pub fn new_read<T: Into<String>>(msg: T) -> Self {
        Self {
            err_type: ErrorType::Read,
            msg: format!("cannot read {}", msg.into()),
        }
    }
}
