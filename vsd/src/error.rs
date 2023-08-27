pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum ErrorKind {
    Input,
    Io,
    Network,
    Other,
    Parse,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    reason: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for Error {}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self {
            kind: ErrorKind::Other,
            reason: value.to_owned(),
        }
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self {
            kind: ErrorKind::Other,
            reason: value,
        }
    }
}

impl From<requestty::ErrorKind> for Error {
    fn from(value: requestty::ErrorKind) -> Self {
        Self {
            kind: ErrorKind::Input,
            reason: format!(
                "user input couldn't be captured. (requestty-error: {})",
                value
            ),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        let url = value.url().map(|x| x.as_str()).unwrap_or("resource");

        if let Some(status) = value.status() {
            if status.is_client_error() || status.is_server_error() {
                return Self {
                    kind: ErrorKind::Network,
                    reason: format!("{} couldn't be reached. (status-code: {})", url, status),
                };
            }
        }

        Self {
            kind: ErrorKind::Network,
            reason: format!("{} request failed. (reqwest-error: {})", url, value),
        }
    }
}

impl From<url::ParseError> for Error {
    fn from(value: url::ParseError) -> Self {
        Self {
            kind: ErrorKind::Parse,
            reason: format!("cannot parse or join urls. (url-error: {})", value),
        }
    }
}

impl Error {
    pub fn new<T: Into<String>>(reason: T) -> Self {
        Self {
            kind: ErrorKind::Other,
            reason: reason.into(),
        }
    }

    pub fn new_io<T: Into<String>>(reason: T) -> Self {
        Self {
            kind: ErrorKind::Io,
            reason: reason.into(),
        }
    }

    pub fn new_parse<T: Into<String>>(reason: T) -> Self {
        Self {
            kind: ErrorKind::Parse,
            reason: reason.into(),
        }
    }
}
