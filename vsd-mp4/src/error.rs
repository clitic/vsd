use thiserror::Error;

/// The returned error type.
#[derive(Debug, Error)]
pub enum Error {
    #[cfg(feature = "decrypt")]
    #[cfg_attr(docsrs, doc(cfg(feature = "decrypt")))]
    #[error("{0}")]
    Decrypt(#[from] crate::decrypt::DecryptError),

    #[error("{0}")]
    Generic(String),

    #[cfg(feature = "pssh")]
    #[cfg_attr(docsrs, doc(cfg(feature = "pssh")))]
    #[error("Failed to decode protobuf: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),

    #[error("Failed to read data: {0}")]
    Read(#[from] std::io::Error),

    #[error("Failed to decode string: {0}")]
    StringDecodeUtf8(#[from] std::string::FromUtf8Error),

    #[error("Failed to decode string: {0}")]
    StringDecodeUtf16(#[from] std::string::FromUtf16Error),

    #[cfg(feature = "pssh")]
    #[cfg_attr(docsrs, doc(cfg(feature = "pssh")))]
    #[error("Failed to decode xml: {error} => {xml}")]
    XmlDecode {
        error: quick_xml::de::DeError,
        xml: String,
    },
}

/// Creates an `Error::Generic` and returns early (like `anyhow::bail!`).
#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::Error::Generic(format!($($arg)*)))
    };
}
