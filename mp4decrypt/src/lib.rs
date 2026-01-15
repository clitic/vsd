//! This crate provides a safe high level api to decrypt mp4 data using [Bento4](https://github.com/axiomatic-systems/Bento4).
//!
//! ## Environment Variables
//!
//! A set of environment variables that can be used to find ap4 library from Bento4 installation.
//!  
//! - BENTO4_DIR - If specified, the directory of an Bento4 installation.
//!   The directory should contain lib and include subdirectories containing the libraries and headers respectively.
//! - BENTO4_VENDOR - If set, always build and link against Bento4 vendored version.
//!
//! Additionally, these variables can be prefixed with the upper-cased target architecture (e.g. X86_64_UNKNOWN_LINUX_GNU_BENTO4_DIR),
//! which can be useful when cross compiling.

mod error;

pub use error::Error;

use core::ffi::{c_int, c_uchar, c_uint, c_void};
use std::{collections::HashMap, fs, path::Path, ptr, sync::Mutex};

// Bento4 has global state that is not thread-safe
static BENTO4_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn ap4_context_new(keys: *const c_uchar, keys_count: c_uint) -> *mut c_void;
    fn ap4_decrypt(
        ctx: *mut c_void,
        data: *const c_uchar,
        data_size: c_uint,
        out_data: *mut *mut c_uchar,
        out_size: *mut c_uint,
    ) -> c_int;
    fn ap4_context_free(ctx: *mut c_void);
    fn ap4_free(ptr: *mut c_uchar);
}

fn verify_hex(input: String) -> Result<[u8; 16], Error> {
    let bytes = hex::decode(&input)?;

    if bytes.len() != 16 {
        return Err(Error::InvalidHex {
            input,
            message: format!("expected 16 bytes got {} bytes", bytes.len()),
        });
    }

    let mut data = [0u8; 16];
    data.copy_from_slice(&bytes);
    Ok(data)
}

/// A reusable MP4 decryption context.
///
/// Create once with keys, then call `decrypt` multiple times on different segments.
/// This is more efficient than using `Mp4Decrypter` when decrypting many segments
/// with the same keys.
///
/// # Example
///
/// ```no_run
/// use mp4decrypt::Ap4Context;
///
/// let init_data = std::fs::read("init.mp4").unwrap();
/// let ctx = Ap4Context::new()
///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
///     .build()?;
///
/// // Decrypt multiple segments with the same context
/// for i in 1..=10 {
///     let segment = std::fs::read(format!("segment_{}.m4s", i)).unwrap();
///     let decrypted = ctx.decrypt(&init_data, &segment)?;
///     std::fs::write(format!("decrypted_{}.m4s", i), decrypted).unwrap();
/// }
/// # Ok::<(), mp4decrypt::Error>(())
/// ```
pub struct Ap4Context {
    ptr: *mut c_void,
}

// Safety: The C++ context is thread-safe for read operations
unsafe impl Send for Ap4Context {}
unsafe impl Sync for Ap4Context {}

impl Ap4Context {
    /// Creates a new context builder.
    pub fn new() -> Ap4ContextBuilder {
        Ap4ContextBuilder {
            keys: HashMap::new(),
        }
    }

    /// Decrypts data using this context.
    ///
    /// # Arguments
    ///
    /// * `init_data` - The initialization segment data (can be empty slice if not needed)
    /// * `segment_data` - The encrypted segment data
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails.
    pub fn decrypt(&self, init_data: &[u8], segment_data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut data = Vec::with_capacity(init_data.len() + segment_data.len());
        data.extend_from_slice(init_data);
        data.extend_from_slice(segment_data);

        let data_size = u32::try_from(data.len()).map_err(|_| Error::DataTooLarge)?;

        let mut out_data: *mut c_uchar = ptr::null_mut();
        let mut out_size: c_uint = 0;

        let result = {
            let _lock = BENTO4_LOCK.lock().unwrap();
            unsafe {
                ap4_decrypt(
                    self.ptr,
                    data.as_ptr(),
                    data_size,
                    &mut out_data,
                    &mut out_size,
                )
            }
        };

        if result == 0 {
            let decrypted = unsafe {
                let slice = std::slice::from_raw_parts(out_data, out_size as usize);
                let vec = slice.to_vec();
                ap4_free(out_data);
                vec
            };
            Ok(decrypted)
        } else {
            Err(Error::DecryptionFailed(result))
        }
    }
}

impl Default for Ap4ContextBuilder {
    fn default() -> Self {
        Ap4Context::new()
    }
}

impl Drop for Ap4Context {
    fn drop(&mut self) {
        let _lock = BENTO4_LOCK.lock().unwrap();
        unsafe { ap4_context_free(self.ptr) }
    }
}

/// Builder for creating an `Ap4Context`.
pub struct Ap4ContextBuilder {
    keys: HashMap<[u8; 16], [u8; 16]>,
}

impl Ap4ContextBuilder {
    /// Adds a key-id and key pair.
    pub fn key(mut self, kid: &str, key: &str) -> Result<Self, Error> {
        self.keys
            .insert(verify_hex(kid.to_owned())?, verify_hex(key.to_owned())?);
        Ok(self)
    }

    /// Adds multiple key-id and key pairs.
    pub fn keys(mut self, keys: HashMap<String, String>) -> Result<Self, Error> {
        for (kid, key) in keys {
            self.keys.insert(verify_hex(kid)?, verify_hex(key)?);
        }
        Ok(self)
    }

    /// Builds the context. Fails if no keys were provided.
    pub fn build(self) -> Result<Ap4Context, Error> {
        if self.keys.is_empty() {
            return Err(Error::NoKeys);
        }

        let keys_count = self.keys.len();
        let mut keys_buffer = Vec::with_capacity(keys_count * 32);

        for (kid, key) in &self.keys {
            keys_buffer.extend_from_slice(kid);
            keys_buffer.extend_from_slice(key);
        }

        let ptr = {
            let _lock = BENTO4_LOCK.lock().unwrap();
            unsafe { ap4_context_new(keys_buffer.as_ptr(), keys_count as c_uint) }
        };

        Ok(Ap4Context { ptr })
    }
}

/// A one-shot builder for decrypting encrypted MP4 streams.
///
/// For decrypting multiple segments with the same keys, prefer using [`Ap4Context`]
/// which is more efficient.
///
/// # Example
///
/// ```no_run
/// use mp4decrypt::Mp4Decrypter;
///
/// let decrypted = Mp4Decrypter::new()
///     .key("eb676abbcb345e96bbcf616630f1a3da", "100b6c20940f779a4589152b57d2dacb")?
///     .init_file("init.mp4")?
///     .input_file("segment.m4s")?
///     .decrypt()?;
/// # Ok::<(), mp4decrypt::Error>(())
/// ```
pub struct Mp4Decrypter {
    keys: HashMap<[u8; 16], [u8; 16]>,
    init_data: Option<Vec<u8>>,
    input_data: Option<Vec<u8>>,
}

impl Mp4Decrypter {
    /// Creates a new `Mp4Decrypter` instance with no keys or data configured.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            init_data: None,
            input_data: None,
        }
    }

    /// Adds a single key-id and key pair for decryption.
    pub fn key(mut self, kid: &str, key: &str) -> Result<Self, Error> {
        self.keys
            .insert(verify_hex(kid.to_owned())?, verify_hex(key.to_owned())?);
        Ok(self)
    }

    /// Adds multiple key-id and key pairs for decryption.
    pub fn keys(mut self, keys: HashMap<String, String>) -> Result<Self, Error> {
        for (kid, key) in keys {
            self.keys.insert(verify_hex(kid)?, verify_hex(key)?);
        }
        Ok(self)
    }

    /// Sets the initialization segment data from a byte vector.
    pub fn init_data(mut self, data: Vec<u8>) -> Self {
        self.init_data = Some(data);
        self
    }

    /// Sets the initialization segment data by reading from a file.
    pub fn init_file(mut self, path: impl AsRef<Path>) -> Result<Self, Error> {
        let path_ref = path.as_ref();
        self.init_data = Some(fs::read(path_ref).map_err(|e| Error::FileRead {
            path: path_ref.display().to_string(),
            source: e,
        })?);
        Ok(self)
    }

    /// Sets the encrypted input data from a byte vector.
    pub fn input_data(mut self, data: Vec<u8>) -> Self {
        self.input_data = Some(data);
        self
    }

    /// Sets the encrypted input data by reading from a file.
    pub fn input_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Error> {
        let path_ref = path.as_ref();
        self.input_data = Some(fs::read(path_ref).map_err(|e| Error::FileRead {
            path: path_ref.display().to_string(),
            source: e,
        })?);
        Ok(self)
    }

    /// Decrypts the configured data and returns the decrypted bytes.
    pub fn decrypt(self) -> Result<Vec<u8>, Error> {
        let ctx = Ap4ContextBuilder { keys: self.keys }.build()?;
        let init_data = self.init_data.unwrap_or_default();
        let input_data = self.input_data.ok_or(Error::NoData)?;
        ctx.decrypt(&init_data, &input_data)
    }

    /// Decrypts the configured data and writes the result to a file.
    pub fn decrypt_to_file(self, path: impl AsRef<Path>) -> Result<(), Error> {
        let data = self.decrypt()?;
        let path_ref = path.as_ref();
        fs::write(path_ref, data).map_err(|e| Error::FileWrite {
            path: path_ref.display().to_string(),
            source: e,
        })?;
        Ok(())
    }
}
