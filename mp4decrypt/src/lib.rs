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

pub use error::{Error, ErrorType};

use core::ffi::{c_char, c_int, c_uchar, c_uint};
use std::{collections::HashMap, ffi::CString};

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

/// Decrypt encrypted mp4 data stream using given keys.
///
/// # Arguments
///
/// * `data` - Encrypted data stream.
/// * `kid_key_pairs` - Hashmap of kid key pairs for decrypting data stream.
///   Hashmap `key` is either a track ID in decimal or a 128-bit KID in hex.
///   Hashmap `value` is a 128-bit key in hex. <br>
///   1. For dcf files, use 1 as the track index <br>
///   2. For Marlin IPMP/ACGK, use 0 as the track ID <br>
///   3. KIDs are only applicable to some encryption methods like MPEG-CENC <br>
/// * `fragments_info` (optional) - Decrypt the fragments read from data stream, with track info read from this stream.
///
/// # Example
///
/// ```no_run
/// use std::collections::HashMap;
///
/// let kid_key_pairs = HashMap::from([(
///     "eb676abbcb345e96bbcf616630f1a3da".to_owned(),
///     "100b6c20940f779a4589152b57d2dacb".to_owned(),
/// )]);
///
/// let decrypted_data = mp4decrypt::mp4decrypt(&[0, 0, 0, 112], &kid_key_pairs, None).unwrap();
/// ```
pub fn mp4decrypt(
    data: &[u8],
    keys: &HashMap<String, String>,
) -> Result<Vec<u8>, Error> {
    let data_size = u32::try_from(data.len()).map_err(|_| Error {
        msg: "the input data stream is too large.".to_owned(),
        err_type: ErrorType::DataTooLarge,
    })?;

    let c_keys = keys
        .iter()
        .map(|(kid, key)| CString::new(format!("{}:{}", kid, key)).unwrap())
        .collect::<Vec<_>>();
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
        Err(match result {
            -999 => Error {
                msg: "invalid argument for keys.".to_owned(),
                err_type: ErrorType::InvalidFormat,
            },
            -998 => Error {
                msg: "invalid hex format for key id.".to_owned(),
                err_type: ErrorType::InvalidFormat,
            },
            -997 => Error {
                msg: "invalid key id.".to_owned(),
                err_type: ErrorType::InvalidFormat,
            },
            -996 => Error {
                msg: "invalid hex format for key.".to_owned(),
                err_type: ErrorType::InvalidFormat,
            },
            x => Error {
                msg: format!("failed to decrypt data with error code {x}."),
                err_type: ErrorType::Failed(x),
            },
        })
    }
}
