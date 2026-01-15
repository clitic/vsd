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

#![allow(improper_ctypes)]

mod error;

pub use error::Error;

use core::ffi::{c_char, c_int, c_uchar, c_uint};
use std::{collections::HashMap, ffi::CString, fs, path::Path};

unsafe extern "C" {
    fn ap4_mp4decrypt(
        data: *const c_uchar,
        data_size: c_uint,
        kid_raw: *const *const c_char,
        key_raw: *const *const c_char,
        keys_size: c_uint,
        decrypted_data: *mut Vec<u8>,
        callback_rust: extern "C" fn(*mut Vec<u8>, *const c_uchar, c_uint),
    ) -> c_int;
}

extern "C" fn callback_rust(decrypted_stream: *mut Vec<u8>, data: *const c_uchar, size: c_uint) {
    unsafe {
        *decrypted_stream = std::slice::from_raw_parts(data, size as usize).to_vec();
    }
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

/// A builder for decrypting encrypted MP4 streams.
///
/// This struct uses a builder pattern to configure decryption parameters
/// including keys, initialization data, and input data before performing
/// the actual decryption.
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
    ///
    /// Both `kid` (key ID) and `key` must be valid hex strings that decode to exactly 16 bytes.
    ///
    /// # Arguments
    ///
    /// * `kid` - The key ID as a 32-character hex string (16 bytes)
    /// * `key` - The decryption key as a 32-character hex string (16 bytes)
    ///
    /// # Errors
    ///
    /// Returns an error if either `kid` or `key` is not valid hex or not 16 bytes.
    pub fn key(mut self, kid: &str, key: &str) -> Result<Self, Error> {
        self.keys
            .insert(verify_hex(kid.to_owned())?, verify_hex(key.to_owned())?);
        Ok(self)
    }

    /// Adds multiple key-id and key pairs for decryption.
    ///
    /// All keys in the map must be valid hex strings that decode to exactly 16 bytes.
    ///
    /// # Arguments
    ///
    /// * `keys` - A map of key IDs to decryption keys, both as hex strings
    ///
    /// # Errors
    ///
    /// Returns an error if any key ID or key is not valid hex or not 16 bytes.
    pub fn keys(mut self, keys: HashMap<String, String>) -> Result<Self, Error> {
        for (kid, key) in keys {
            self.keys.insert(verify_hex(kid)?, verify_hex(key)?);
        }
        Ok(self)
    }

    /// Sets the initialization segment data from a byte vector.
    ///
    /// The initialization segment (often `init.mp4`) contains the encryption
    /// metadata needed to decrypt the media segments.
    pub fn init_data(mut self, data: Vec<u8>) -> Self {
        self.init_data = Some(data);
        self
    }

    /// Sets the initialization segment data by reading from a file.
    ///
    /// The initialization segment (often `init.mp4`) contains the encryption
    /// metadata needed to decrypt the media segments.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn init_file(mut self, path: impl AsRef<Path>) -> Result<Self, Error> {
        let path_ref = path.as_ref();
        self.init_data = Some(fs::read(path_ref).map_err(|e| Error::FileRead {
            path: path_ref.display().to_string(),
            source: e,
        })?);
        Ok(self)
    }

    /// Sets the encrypted input data from a byte vector.
    ///
    /// This is the encrypted media segment data that will be decrypted.
    pub fn input_data(mut self, data: Vec<u8>) -> Self {
        self.input_data = Some(data);
        self
    }

    /// Sets the encrypted input data by reading from a file.
    ///
    /// This is the encrypted media segment file that will be decrypted.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn input_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Error> {
        let path_ref = path.as_ref();
        self.input_data = Some(fs::read(path_ref).map_err(|e| Error::FileRead {
            path: path_ref.display().to_string(),
            source: e,
        })?);
        Ok(self)
    }

    /// Decrypts the configured data and returns the decrypted bytes.
    ///
    /// This method consumes the builder and performs the actual decryption
    /// using the configured keys and data. If initialization data was provided,
    /// it will be prepended to the input data before decryption.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No keys were configured ([`Error::NoKeys`])
    /// - No input data was provided ([`Error::NoData`])
    /// - The combined data exceeds the maximum size ([`Error::DataTooLarge`])
    /// - The decryption fails ([`Error::DecryptionFailed`])
    pub fn decrypt(self) -> Result<Vec<u8>, Error> {
        if self.keys.is_empty() {
            return Err(Error::NoKeys);
        }

        let mut data = Vec::new();

        if let Some(mut init_data) = self.init_data {
            data.append(&mut init_data);
        }

        let input_data = self.input_data.ok_or(Error::NoData)?;
        data.extend(input_data);

        let data_size = u32::try_from(data.len()).map_err(|_| Error::DataTooLarge)?;

        let (kid_raw, key_raw): (Vec<CString>, Vec<CString>) = self
            .keys
            .iter()
            .map(|(kid, key)| {
                (
                    CString::new(hex::encode(kid)).unwrap(),
                    CString::new(hex::encode(key)).unwrap(),
                )
            })
            .unzip();

        let kid_ptr: Vec<*const c_char> = kid_raw.iter().map(|s| s.as_ptr()).collect();
        let key_ptr: Vec<*const c_char> = key_raw.iter().map(|s| s.as_ptr()).collect();
        let mut decrypted_data: Box<Vec<u8>> = Box::default();

        let result = unsafe {
            ap4_mp4decrypt(
                data.as_ptr(),
                data_size,
                kid_ptr.as_ptr(),
                key_ptr.as_ptr(),
                kid_ptr.len() as c_uint,
                &mut *decrypted_data,
                callback_rust,
            )
        };

        if result == 0 {
            Ok(*decrypted_data)
        } else {
            Err(Error::DecryptionFailed(result))
        }
    }

    /// Decrypts the configured data and writes the result to a file.
    ///
    /// This is a convenience method that combines [`decrypt`](Self::decrypt)
    /// with writing the result to a file.
    ///
    /// # Errors
    ///
    /// Returns an error if decryption fails or if the file cannot be written.
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
