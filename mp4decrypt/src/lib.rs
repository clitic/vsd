//! This crate provides a safe high-level API to decrypt CENC/CBCS encrypted MP4 data
//! using [Bento4](https://github.com/axiomatic-systems/Bento4).
//!
//! ## Environment Variables
//!
//! The following environment variables can be used to configure the Bento4 library location:
//!
//! | Variable | Description |
//! |----------|-------------|
//! | `BENTO4_DIR` | Directory of a Bento4 installation. Should contain `lib` and `include` subdirectories. |
//! | `BENTO4_VENDOR` | If set, always build and link against the vendored Bento4 version. |
//!
//! These variables can also be prefixed with the upper-cased target architecture
//! (e.g. `X86_64_UNKNOWN_LINUX_GNU_BENTO4_DIR`), which is useful for cross-compilation.
//!
//! ## Quick Start
//!
//! ```no_run
//! use mp4decrypt::Ap4CencDecryptingProcessor;
//! use std::{error::Error, fs};
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     // Create a processor with decryption keys
//!     let processor = Ap4CencDecryptingProcessor::new()
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
//! ## Multithreaded Decryption
//!
//! The processor is thread-safe and can be shared across multiple threads using `Arc`.
//! This is useful for decrypting multiple segments in parallel:
//!
//! ```no_run
//! use mp4decrypt::Ap4CencDecryptingProcessor;
//! use std::{fs, sync::Arc, thread};
//!
//! fn main() -> Result<(), mp4decrypt::Error> {
//!     // Create a shared processor
//!     let processor = Arc::new(
//!         Ap4CencDecryptingProcessor::new()
//!             .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
//!             .build()?
//!     );
//!
//!     let init_data = Arc::new(fs::read("init.mp4").unwrap());
//!
//!     // Spawn multiple threads to decrypt segments in parallel
//!     let handles: Vec<_> = (1..=10)
//!         .map(|i| {
//!             let processor = Arc::clone(&processor);
//!             let init = Arc::clone(&init_data);
//!
//!             thread::spawn(move || {
//!                 let segment = fs::read(format!("segment_{}.m4s", i)).unwrap();
//!                 let decrypted = processor.decrypt(&segment, Some(init.as_slice())).unwrap();
//!                 fs::write(format!("decrypted_{}.mp4", i), decrypted).unwrap();
//!             })
//!         })
//!         .collect();
//!
//!     // Wait for all threads to complete
//!     for handle in handles {
//!         handle.join().expect("Thread panicked");
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## File-Based Decryption
//!
//! For large files or when you want to avoid loading everything into memory:
//!
//! ```no_run
//! use mp4decrypt::Ap4CencDecryptingProcessor;
//!
//! fn main() -> Result<(), mp4decrypt::Error> {
//!     let processor = Ap4CencDecryptingProcessor::new()
//!         .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
//!         .build()?;
//!
//!     // Decrypt directly from file to file
//!     processor.decrypt_file(
//!         "encrypted_segment.m4s",
//!         "decrypted_segment.m4s",
//!         Some("init.mp4"),
//!     )?;
//!
//!     Ok(())
//! }
//! ```

mod error;

pub use error::Error;

use core::ffi::{c_char, c_int, c_uchar, c_uint, c_void};
use std::{collections::HashMap, ffi::CString, path::Path, ptr, sync::Mutex};

static AP4_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn ap4_processor_new(keys: *const c_uchar, size: c_uint) -> *mut c_void;
    fn ap4_processor_free(ctx: *mut c_void);
    fn ap4_free(ptr: *mut c_uchar);
    fn ap4_decrypt_file(
        ctx: *mut c_void,
        input_path: *const c_char,
        output_path: *const c_char,
        init_path: *const c_char,
    ) -> c_int;
    fn ap4_decrypt_memory(
        ctx: *mut c_void,
        input_data: *const c_uchar,
        input_size: c_uint,
        output_data: *mut *mut c_uchar,
        output_size: *mut c_uint,
    ) -> c_int;
}

/// A CENC (Common Encryption) decrypting processor for MP4 files.
///
/// This struct wraps the Bento4 `AP4_CencDecryptingProcessor` and provides
/// a safe Rust interface for decrypting CENC or CBCS encrypted MP4 content.
///
/// # Thread Safety
///
/// `Ap4CencDecryptingProcessor` implements both `Send` and `Sync`, meaning it can be
/// safely shared across threads using `Arc<Ap4CencDecryptingProcessor>`. Internally,
/// all decryption operations are synchronized using a mutex to ensure thread safety.
///
/// # Example
///
/// ```no_run
/// use mp4decrypt::Ap4CencDecryptingProcessor;
/// use std::{fs, sync::Arc, thread};
///
/// let processor = Arc::new(
///     Ap4CencDecryptingProcessor::new()
///         .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")
///         .unwrap()
///         .build()
///         .unwrap()
/// );
///
/// // Clone Arc for use in another thread
/// let processor_clone = Arc::clone(&processor);
/// let handle = thread::spawn(move || {
///     // Use processor_clone in this thread
/// });
/// ```
pub struct Ap4CencDecryptingProcessor {
    ptr: *mut c_void,
}

// SAFETY: The underlying Bento4 processor is protected by AP4_LOCK mutex,
// making all operations thread-safe.
unsafe impl Send for Ap4CencDecryptingProcessor {}
unsafe impl Sync for Ap4CencDecryptingProcessor {}

impl Ap4CencDecryptingProcessor {
    /// Creates a new builder for configuring the decryption processor.
    ///
    /// Use the builder to add one or more KID/key pairs, then call [`Ap4CencDecryptingProcessorBuilder::build`]
    /// to create the processor.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mp4decrypt::Ap4CencDecryptingProcessor;
    ///
    /// let processor = Ap4CencDecryptingProcessor::new()
    ///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
    ///     .build()?;
    /// # Ok::<(), mp4decrypt::Error>(())
    /// ```
    pub fn new() -> Ap4CencDecryptingProcessorBuilder {
        Ap4CencDecryptingProcessorBuilder::default()
    }

    /// Decrypts encrypted MP4 data in memory.
    ///
    /// This method takes encrypted segment data and an optional initialization segment,
    /// merges them together, and returns the decrypted MP4 data as a playable file.
    ///
    /// # Arguments
    ///
    /// * `input_data` - The encrypted MP4 segment data (e.g., `.m4s` fragment)
    /// * `init_data` - Optional initialization segment data (e.g., `init.mp4`). When provided,
    ///   the init data is prepended to the input, resulting in a complete playable MP4.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<u8>)` containing the decrypted data on success, or an error if:
    /// - The combined data exceeds 4GB ([`Error::DataTooLarge`])
    /// - Decryption fails ([`Error::DecryptionFailed`])
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mp4decrypt::Ap4CencDecryptingProcessor;
    /// use std::fs;
    ///
    /// let processor = Ap4CencDecryptingProcessor::new()
    ///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
    ///     .build()?;
    ///
    /// // With initialization segment (produces playable MP4)
    /// let init = fs::read("video_init.mp4")?;
    /// let segment = fs::read("video_1.m4s")?;
    /// let decrypted = processor.decrypt(&segment, Some(&init))?;
    /// fs::write("output.mp4", decrypted)?;
    ///
    /// // Without initialization segment (raw decrypted segment)
    /// let decrypted_raw = processor.decrypt(&segment, None::<&[u8]>)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads
    /// when the processor is shared via `Arc`.
    pub fn decrypt<T: AsRef<[u8]>>(
        &self,
        input_data: T,
        init_data: Option<T>,
    ) -> Result<Vec<u8>, Error> {
        let mut data = Vec::with_capacity(
            init_data.as_ref().map_or(0, |x| x.as_ref().len()) + input_data.as_ref().len(),
        );

        if let Some(init_data) = init_data {
            data.extend_from_slice(init_data.as_ref());
        }

        data.extend_from_slice(input_data.as_ref());

        let data_size = u32::try_from(data.len()).map_err(|_| Error::DataTooLarge)?;

        let mut output_data: *mut c_uchar = ptr::null_mut();
        let mut output_size: c_uint = 0;

        let result = {
            let _lock = AP4_LOCK.lock().unwrap();
            unsafe {
                ap4_decrypt_memory(
                    self.ptr,
                    data.as_ptr(),
                    data_size,
                    &mut output_data,
                    &mut output_size,
                )
            }
        };

        if result == 0 {
            let decrypted = unsafe {
                let slice = std::slice::from_raw_parts(output_data, output_size as usize);
                let vec = slice.to_vec();
                ap4_free(output_data);
                vec
            };
            Ok(decrypted)
        } else {
            Err(Error::DecryptionFailed(result))
        }
    }

    /// Decrypts an encrypted MP4 file and writes the result to disk.
    ///
    /// This method is more memory-efficient than [`decrypt`](Self::decrypt) for large files
    /// as it streams data directly from disk rather than loading it all into memory.
    ///
    /// # Arguments
    ///
    /// * `input_path` - Path to the encrypted MP4 segment file (e.g., `segment.m4s`)
    /// * `output_path` - Path where the decrypted data will be written
    /// * `init_path` - Optional path to the initialization segment (e.g., `init.mp4`).
    ///   Required for proper decryption of fragmented MP4 files.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if decryption fails.
    ///
    /// # Note
    ///
    /// Unlike [`decrypt`](Self::decrypt), this method does **not** combine the init and
    /// segment data in the output. If you need a playable MP4 file, you must manually
    /// concatenate the init segment with the decrypted output:
    ///
    /// ```no_run
    /// use mp4decrypt::Ap4CencDecryptingProcessor;
    /// use std::fs;
    ///
    /// let processor = Ap4CencDecryptingProcessor::new()
    ///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
    ///     .build()?;
    ///
    /// // Decrypt the segment
    /// processor.decrypt_file(
    ///     "encrypted_segment.m4s",
    ///     "decrypted_segment.m4s",
    ///     Some("init.mp4"),
    /// )?;
    ///
    /// // Create playable MP4 by concatenating init + decrypted segment
    /// let init = fs::read("init.mp4")?;
    /// let segment = fs::read("decrypted_segment.m4s")?;
    /// let mut playable = init;
    /// playable.extend(segment);
    /// fs::write("playable.mp4", playable)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads
    /// when the processor is shared via `Arc`.
    pub fn decrypt_file<T: AsRef<Path>>(
        &self,
        input_path: T,
        output_path: T,
        init_path: Option<T>,
    ) -> Result<(), Error> {
        let input_cstr = CString::new(input_path.as_ref().to_string_lossy().as_bytes()).unwrap();
        let output_cstr = CString::new(output_path.as_ref().to_string_lossy().as_bytes()).unwrap();
        let init_cstr =
            init_path.map(|p| CString::new(p.as_ref().to_string_lossy().as_bytes()).unwrap());

        let init_ptr = init_cstr
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());

        let result = {
            let _lock = AP4_LOCK.lock().unwrap();
            unsafe {
                ap4_decrypt_file(
                    self.ptr,
                    input_cstr.as_ptr(),
                    output_cstr.as_ptr(),
                    init_ptr,
                )
            }
        };

        if result == 0 {
            Ok(())
        } else {
            Err(Error::DecryptionFailed(result))
        }
    }
}

impl Drop for Ap4CencDecryptingProcessor {
    fn drop(&mut self) {
        let _lock = AP4_LOCK.lock().unwrap();
        unsafe { ap4_processor_free(self.ptr) }
    }
}

impl Default for Ap4CencDecryptingProcessorBuilder {
    fn default() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }
}

/// Builder for creating [`Ap4CencDecryptingProcessor`] instances.
///
/// Use this builder to configure decryption keys before creating the processor.
/// At least one KID/key pair must be provided.
///
/// # Example
///
/// ```no_run
/// use mp4decrypt::Ap4CencDecryptingProcessor;
/// use std::collections::HashMap;
///
/// // Add keys one at a time
/// let processor = Ap4CencDecryptingProcessor::new()
///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
///     .key("63cb5f7184dd4b689a5c5ff11ee6a328", "3bda3329158a4789880816a70e7e436d")?
///     .build()?;
///
/// // Or add multiple keys from a HashMap
/// let mut keys = HashMap::new();
/// keys.insert(
///     "eb676abbcb345e96bbcf616630f1a3da".to_string(),
///     "100b6c20940f779a4589152b57d2dacb".to_string(),
/// );
/// let processor = Ap4CencDecryptingProcessor::new()
///     .keys(&keys)?
///     .build()?;
/// # Ok::<(), mp4decrypt::Error>(())
/// ```
pub struct Ap4CencDecryptingProcessorBuilder {
    keys: HashMap<[u8; 16], [u8; 16]>,
}

impl Ap4CencDecryptingProcessorBuilder {
    /// Adds a single KID/key pair for decryption.
    ///
    /// # Arguments
    ///
    /// * `kid` - The Key ID as a 32-character hexadecimal string (16 bytes)
    /// * `key` - The decryption key as a 32-character hexadecimal string (16 bytes)
    ///
    /// # Returns
    ///
    /// Returns `Ok(Self)` for method chaining, or an error if:
    /// - The hex string is invalid ([`Error::HexDecode`])
    /// - The hex string doesn't represent exactly 16 bytes ([`Error::InvalidHex`])
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mp4decrypt::Ap4CencDecryptingProcessor;
    ///
    /// let processor = Ap4CencDecryptingProcessor::new()
    ///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
    ///     .build()?;
    /// # Ok::<(), mp4decrypt::Error>(())
    /// ```
    pub fn key(mut self, kid: &str, key: &str) -> Result<Self, Error> {
        self.keys.insert(verify_hex(kid)?, verify_hex(key)?);
        Ok(self)
    }

    /// Adds multiple KID/key pairs from a HashMap.
    ///
    /// This is a convenience method for adding multiple keys at once,
    /// useful when keys are loaded from configuration files or external sources.
    ///
    /// # Arguments
    ///
    /// * `keys` - A HashMap where keys are KIDs and values are decryption keys,
    ///   both as 32-character hexadecimal strings
    ///
    /// # Returns
    ///
    /// Returns `Ok(Self)` for method chaining, or an error if any hex string is invalid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mp4decrypt::Ap4CencDecryptingProcessor;
    /// use std::collections::HashMap;
    ///
    /// let mut keys = HashMap::new();
    /// keys.insert(
    ///     "eb676abbcb345e96bbcf616630f1a3da".to_string(),
    ///     "100b6c20940f779a4589152b57d2dacb".to_string(),
    /// );
    /// keys.insert(
    ///     "63cb5f7184dd4b689a5c5ff11ee6a328".to_string(),
    ///     "3bda3329158a4789880816a70e7e436d".to_string(),
    /// );
    ///
    /// let processor = Ap4CencDecryptingProcessor::new()
    ///     .keys(&keys)?
    ///     .build()?;
    /// # Ok::<(), mp4decrypt::Error>(())
    /// ```
    pub fn keys(mut self, keys: &HashMap<String, String>) -> Result<Self, Error> {
        for (kid, key) in keys {
            self.keys.insert(verify_hex(kid)?, verify_hex(key)?);
        }
        Ok(self)
    }

    /// Builds the [`Ap4CencDecryptingProcessor`] with the configured keys.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Ap4CencDecryptingProcessor)` on success, or [`Error::NoKeys`]
    /// if no keys were provided.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mp4decrypt::Ap4CencDecryptingProcessor;
    ///
    /// let processor = Ap4CencDecryptingProcessor::new()
    ///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
    ///     .build()?;
    /// # Ok::<(), mp4decrypt::Error>(())
    /// ```
    pub fn build(self) -> Result<Ap4CencDecryptingProcessor, Error> {
        if self.keys.is_empty() {
            return Err(Error::NoKeys);
        }

        let size = self.keys.len();
        let mut buffer = Vec::with_capacity(size * 32);

        for (kid, key) in &self.keys {
            buffer.extend_from_slice(kid);
            buffer.extend_from_slice(key);
        }

        let ptr = {
            let _lock = AP4_LOCK.lock().unwrap();
            unsafe { ap4_processor_new(buffer.as_ptr(), size as c_uint) }
        };

        Ok(Ap4CencDecryptingProcessor { ptr })
    }
}

fn verify_hex(input: &str) -> Result<[u8; 16], Error> {
    let bytes = hex::decode(input)?;

    if bytes.len() != 16 {
        return Err(Error::InvalidHex {
            input: input.to_owned(),
            message: format!("expected 16 bytes got {} bytes", bytes.len()),
        });
    }

    let mut data = [0u8; 16];
    data.copy_from_slice(&bytes);
    Ok(data)
}
