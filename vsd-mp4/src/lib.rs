mod error;
mod reader;

pub mod boxes;
pub mod parser;
pub mod pssh;
pub mod text;

pub use error::Error;
pub use reader::Reader;

/// A `Result` alias where the `Err` case is `vsd_mp4::Error`.
pub type Result<T> = std::result::Result<T, Error>;
