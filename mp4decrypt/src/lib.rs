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

pub struct Ap4CencDecryptingProcessor {
    ptr: *mut c_void,
}

unsafe impl Send for Ap4CencDecryptingProcessor {}
unsafe impl Sync for Ap4CencDecryptingProcessor {}

impl Ap4CencDecryptingProcessor {
    pub fn new() -> Ap4CencDecryptingProcessorBuilder {
        Ap4CencDecryptingProcessorBuilder::default()
    }

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

pub struct Ap4CencDecryptingProcessorBuilder {
    keys: HashMap<[u8; 16], [u8; 16]>,
}

impl Ap4CencDecryptingProcessorBuilder {
    pub fn key(mut self, kid: &str, key: &str) -> Result<Self, Error> {
        self.keys.insert(verify_hex(kid)?, verify_hex(key)?);
        Ok(self)
    }

    pub fn keys(mut self, keys: &HashMap<String, String>) -> Result<Self, Error> {
        for (kid, key) in keys {
            self.keys.insert(verify_hex(kid)?, verify_hex(key)?);
        }
        Ok(self)
    }

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
