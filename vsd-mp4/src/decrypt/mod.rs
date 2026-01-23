//! CENC/CBCS decryption for MP4 data.
//!
//! This module provides native Rust decryption for Common Encryption (CENC)
//! and Sample-based AES-CBC encryption (CBCS) protected MP4 content.
//!
//! # Supported Encryption Schemes
//!
//! | Scheme | Description | Cipher Mode |
//! |--------|-------------|-------------|
//! | `cenc` | AES-CTR full sample encryption | AES-128-CTR |
//! | `cens` | AES-CTR subsample encryption | AES-128-CTR |
//! | `cbc1` | AES-CBC full sample encryption | AES-128-CBC |
//! | `cbcs` | AES-CBC pattern encryption | AES-128-CBC (1:9 pattern) |
//!
//! # Quick Start
//!
//! ```no_run
//! use vsd_mp4::decrypt::CencDecryptingProcessor;
//! use std::fs;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let processor = CencDecryptingProcessor::builder()
//!         .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
//!         .build()?;
//!
//!     let init_data = fs::read("init.mp4")?;
//!     let segment_data = fs::read("segment.m4s")?;
//!     let decrypted = processor.decrypt(&segment_data, Some(&init_data))?;
//!     fs::write("output.mp4", decrypted)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Session API (Recommended for Multiple Segments)
//!
//! Parse init once, decrypt multiple segments efficiently:
//!
//! ```no_run
//! use vsd_mp4::decrypt::CencDecryptingProcessor;
//! use std::fs;
//!
//! let processor = CencDecryptingProcessor::builder()
//!     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
//!     .build()?;
//!
//! // Parse init once
//! let init = fs::read("init.mp4")?;
//! let session = processor.session(&init)?;
//!
//! // Decrypt multiple segments (no re-parsing)
//! for i in 1..=10 {
//!     let segment = fs::read(format!("segment_{}.m4s", i))?;
//!     let decrypted = session.decrypt(&segment)?;
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod cipher;
mod decrypter;
mod error;
mod processor;
mod sample_info;

pub use cipher::CipherMode;
pub use decrypter::SingleSampleDecrypter;
pub use error::{DecryptError, Result};
pub use processor::{CencDecryptingProcessor, CencDecryptingProcessorBuilder, DecryptionSession};
pub use sample_info::{SampleInfoTable, SubsampleEntry};
