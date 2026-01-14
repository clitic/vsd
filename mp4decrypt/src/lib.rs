//! This crate provides a safe function to decrypt,
//! encrypted mp4 data stream using [Bento4](https://github.com/axiomatic-systems/Bento4).
//!
//! Maximum supported stream size is around `4.29` G.B i.e. [u32::MAX](u32::MAX).
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
        keys: *mut *const c_char,
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

fn verify_hex(input: String) -> Result<String, Error> {
    let bytes = hex::decode(&input)?;

    if bytes.len() != 16 {
        return Err(Error::InvalidHex {
            input,
            message: format!("expected 16 bytes got {} bytes", bytes.len()),
        });
    }

    Ok(input)
}

pub struct Mp4Decrypter {
    keys: HashMap<String, String>,
    init_data: Option<Vec<u8>>,
    input_data: Option<Vec<u8>>,
}

impl Clone for Mp4Decrypter {
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            init_data: None,
            input_data: None,
        }
    }
}

impl Mp4Decrypter {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            init_data: None,
            input_data: None,
        }
    }

    pub fn key(mut self, kid: &str, key: &str) -> Result<Self, Error> {
        self.keys
            .insert(verify_hex(kid.to_owned())?, verify_hex(key.to_owned())?);
        Ok(self)
    }

    pub fn keys(mut self, keys: HashMap<String, String>) -> Result<Self, Error> {
        for (kid, key) in keys {
            self.keys.insert(verify_hex(kid)?, verify_hex(key)?);
        }
        Ok(self)
    }

    pub fn init_data(mut self, data: Vec<u8>) -> Self {
        self.init_data = Some(data);
        self
    }

    pub fn init_file(mut self, path: impl AsRef<Path>) -> Result<Self, Error> {
        let path_ref = path.as_ref();
        self.init_data = Some(fs::read(path_ref).map_err(|e| Error::FileRead {
            path: path_ref.display().to_string(),
            source: e,
        })?);
        Ok(self)
    }

    pub fn input_data(mut self, data: Vec<u8>) -> Self {
        self.input_data = Some(data);
        self
    }

    pub fn input_file<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Error> {
        let path_ref = path.as_ref();
        self.input_data = Some(fs::read(path_ref).map_err(|e| Error::FileRead {
            path: path_ref.display().to_string(),
            source: e,
        })?);
        Ok(self)
    }

    pub fn decrypt(self) -> Result<Vec<u8>, Error> {
        if self.keys.is_empty() {
            return Err(Error::NoKeys);
        }

        let mut data = Vec::new();

        if let Some(mut init_data) = self.init_data {
            data.append(&mut init_data);
        }

        let input_data = self.input_data.ok_or(Error::NoInputData)?;
        data.extend(input_data);

        let data_size = u32::try_from(data.len()).map_err(|_| Error::DataTooLarge)?;

        let c_keys = self
            .keys
            .iter()
            .map(|(kid, key)| {
                CString::new(format!("{}:{}", kid, key)).map_err(|_| Error::FfiString)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut c_keys = c_keys.iter().map(|x| x.as_ptr()).collect::<Vec<_>>();
        let mut decrypted_data: Box<Vec<u8>> = Box::default();

        let result = unsafe {
            ap4_mp4decrypt(
                data.as_ptr(),
                data_size,
                c_keys.as_mut_ptr(),
                c_keys.len() as u32,
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
