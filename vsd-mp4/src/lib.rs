#![cfg_attr(docsrs, feature(doc_cfg))]

//! This crate contains a mp4 parser ported from [shaka-player](https://github.com/shaka-project/shaka-player) project.
//! Also, some optional features are added for decryption, parsing subtitles, `PSSH`, and `SIDX` boxes.
//!
//! # Optional Features
//!
//! The following are a list of [Cargo features](https://doc.rust-lang.org/stable/cargo/reference/features.html#the-features-section) that can be
//! enabled or disabled:
//!
//! - **decrypt**: Enables support for decryption.
//! - **pssh**: Enables support for parsing `PSSH` boxes.
//! - **sidx**: Enables support for parsing `SIDX` boxes.
//! - **text-ttml**: Enables support for extracting ttml subtitles.
//! - **text-vtt**: Enables support for extracting vtt subtitles.

#[cfg(any(feature = "decrypt", feature = "sidx", feature = "text-vtt"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "decrypt", feature = "sidx", feature = "text-vtt")))
)]
pub mod boxes;

#[cfg(feature = "decrypt")]
#[cfg_attr(docsrs, doc(cfg(feature = "decrypt")))]
pub mod decrypt;

#[cfg(feature = "pssh")]
#[cfg_attr(docsrs, doc(cfg(feature = "pssh")))]
pub mod pssh;

#[cfg(any(feature = "text-ttml", feature = "text-vtt"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "text-ttml", feature = "text-vtt"))))]
pub mod text;

mod error;
mod parser;
mod reader;

pub use error::Error;
pub use parser::*;
pub use reader::Reader;

/// A `Result` alias where the `Err` case is `vsd_mp4::Error`.
pub type Result<T> = std::result::Result<T, Error>;

pub type Mp4Box<T> = std::rc::Rc<std::cell::RefCell<Option<T>>>;
