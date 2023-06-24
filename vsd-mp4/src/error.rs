/// The Errors that may occur when parsing some data.
#[derive(Debug)]
pub struct Error {
    read_err: bool,
    reason: String,
    decode_err: bool,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix_reason = if self.read_err {
            "Cannot read "
        } else if self.decode_err {
            "Cannot decode "
        } else {
            ""
        };

        write!(f, "{}{}.", prefix_reason, self.reason)
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Create a new uncategorized error.
    pub fn new<T: Into<String>>(reason: T) -> Self {
        Self {
            read_err: false,
            reason: reason.into(),
            decode_err: false,
        }
    }

    /// Create a new read error.
    pub fn new_read_err<T: Into<String>>(reason: T) -> Self {
        Self {
            read_err: true,
            reason: reason.into(),
            decode_err: false,
        }
    }

    /// Create a new read error.
    pub fn new_decode_err<T: Into<String>>(reason: T) -> Self {
        Self {
            read_err: false,
            reason: reason.into(),
            decode_err: true,
        }
    }

    /// Returns true if the error is a read error.
    pub fn is_read_err(&self) -> bool {
        self.read_err
    }

    /// Returns true if the error is a decode error.
    pub fn is_decode_err(&self) -> bool {
        self.decode_err
    }
}
