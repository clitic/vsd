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
//!     // Create a processor with decryption keys
//!     let processor = CencDecryptingProcessor::builder()
//!         .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
//!         .build()?;
//!
//!     // Decrypt in memory
//!     let init_data = fs::read("init.mp4")?;
//!     let segment_data = fs::read("segment.m4s")?;
//!     let decrypted = processor.decrypt(&segment_data, Some(&init_data))?;
//!     fs::write("output.mp4", decrypted)?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Multi-Key Content
//!
//! For content with separate keys for audio and video tracks:
//!
//! ```no_run
//! use vsd_mp4::decrypt::CencDecryptingProcessor;
//!
//! let processor = CencDecryptingProcessor::builder()
//!     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")? // video
//!     .key("63cb5f7184dd4b689a5c5ff11ee6a328", "3bda3329158a4789880816a70e7e436d")? // audio
//!     .build()?;
//! # Ok::<(), vsd_mp4::decrypt::DecryptError>(())
//! ```
//!
//! # HLS/DASH Segment Decryption
//!
//! For fragmented MP4 streams (fMP4), pass the initialization segment:
//!
//! ```no_run
//! use vsd_mp4::decrypt::CencDecryptingProcessor;
//! use std::fs;
//!
//! let processor = CencDecryptingProcessor::builder()
//!     .key("kid_hex_32_chars", "key_hex_32_chars")?
//!     .build()?;
//!
//! // Read init segment once
//! let init = fs::read("init.mp4")?;
//!
//! // Decrypt multiple segments
//! for i in 1..=10 {
//!     let segment = fs::read(format!("segment_{}.m4s", i))?;
//!     let decrypted = processor.decrypt(&segment, Some(&init))?;
//!     // Process decrypted segment...
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Thread Safety
//!
//! [`CencDecryptingProcessor`] is `Send + Sync` and can be shared across threads
//! using `Arc`. Each call to `decrypt` creates its own internal state.
//!
//! ```no_run
//! use vsd_mp4::decrypt::CencDecryptingProcessor;
//! use std::{sync::Arc, thread};
//!
//! let processor = Arc::new(
//!     CencDecryptingProcessor::builder()
//!         .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")
//!         .unwrap()
//!         .build()
//!         .unwrap()
//! );
//!
//! let handles: Vec<_> = (0..4)
//!     .map(|_| {
//!         let processor = Arc::clone(&processor);
//!         thread::spawn(move || {
//!             // Each thread can decrypt independently
//!         })
//!     })
//!     .collect();
//! ```

mod cipher;
mod decrypter;
mod error;
mod processor;
mod sample_info;

pub use cipher::CipherMode;
pub use decrypter::SingleSampleDecrypter;
pub use error::{DecryptError, Result};
pub use processor::{CencDecryptingProcessor, CencDecryptingProcessorBuilder};
pub use sample_info::{SampleInfoTable, SubsampleEntry};
