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
//! The directory should contain lib and include subdirectories containing the libraries and headers respectively.
//! - BENTO4_VENDOR - If set, always build and link against Bento4 vendored version.
//! 
//! Additionally, these variables can be prefixed with the upper-cased target architecture (e.g. X86_64_UNKNOWN_LINUX_GNU_BENTO4_DIR), 
//! which can be useful when cross compiling.

#![allow(improper_ctypes)]

use core::ffi::{c_char, c_int, c_uchar, c_uint};
use std::collections::HashMap;
use std::ffi::CString;

extern "C" {
    fn decrypt_in_memory(
        data: *const c_uchar,
        data_size: c_uint,
        keyids: *mut *const c_char,
        keys: *mut *const c_char,
        nkeys: c_int,
        decrypted_data: *mut Vec<u8>,
        callback: extern "C" fn(*mut Vec<u8>, *const c_uchar, c_uint),
    ) -> c_int;

    fn decrypt_in_memory_with_fragments_info(
        data: *const c_uchar,
        data_length: c_uint,
        keyids: *mut *const c_char,
        keys: *mut *const c_char,
        nkeys: c_int,
        decrypted_data: *mut Vec<u8>,
        callback: extern "C" fn(*mut Vec<u8>, *const c_uchar, c_uint),
        fragments_info_data: *const c_uchar,
        fragments_info_data_size: c_uint,
    ) -> c_int;
}

extern "C" fn decrypt_callback(decrypted_stream: *mut Vec<u8>, data: *const c_uchar, size: c_uint) {
    unsafe {
        *decrypted_stream = std::slice::from_raw_parts(data, size as usize).to_vec();
    }
}

/// Decrypt encrypted mp4 data stream using given keys.
///
/// # Arguments
///
/// * `data` - Encrypted data stream.
/// * `keys` - Hashmap of keys for decrypting data stream.
///            Hashmap `key` is either a track ID in decimal or a 128-bit KID in hex.
///            Hashmap `value` is a 128-bit key in hex. <br>
///            1. For dcf files, use 1 as the track index <br>
///            2. For Marlin IPMP/ACGK, use 0 as the track ID <br>
///            3. KIDs are only applicable to some encryption methods like MPEG-CENC <br>
/// * `fragments_info` (optional) - Decrypt the fragments read from data stream, with track info read from this stream.
///
/// # Example
///
/// ```no_run
/// use std::collections::HashMap;
///
/// let keys = HashMap::from([(
///     "eb676abbcb345e96bbcf616630f1a3da".to_owned(),
///     "100b6c20940f779a4589152b57d2dacb".to_owned(),
/// )]);
///
/// let decrypted_data = mp4decrypt::mp4decrypt(&[0, 0, 0, 112], keys, None).unwrap();
/// ```
pub fn mp4decrypt(
    data: &[u8],
    keys: HashMap<String, String>,
    fragments_info: Option<Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let mut data = data.to_vec();
    let data_size = u32::try_from(data.len()).map_err(|_| "data stream is too large".to_owned())?;

    let mut c_kids_holder = vec![];
    let mut c_keys_holder = vec![];
    let mut c_kids = vec![];
    let mut c_keys = vec![];

    for (i, (kid, key)) in keys.iter().enumerate() {
        c_kids_holder.push(CString::new(kid.to_owned()).unwrap());
        c_keys_holder.push(CString::new(key.to_owned()).unwrap());
        c_kids.push(c_kids_holder[i].as_ptr());
        c_keys.push(c_keys_holder[i].as_ptr());
    }

    let mut decrypted_data: Box<Vec<u8>> = Box::default();

    let result = unsafe {
        if let Some(mut fragments_info_data) = fragments_info {
            let fragments_info_data_size = u32::try_from(fragments_info_data.len())
                .map_err(|_| "fragments info is too large".to_owned())?;

            decrypt_in_memory_with_fragments_info(
                data.as_mut_ptr(),
                data_size,
                c_kids.as_mut_ptr(),
                c_keys.as_mut_ptr(),
                1,
                &mut *decrypted_data,
                decrypt_callback,
                fragments_info_data.as_mut_ptr(),
                fragments_info_data_size,
            )
        } else {
            decrypt_in_memory(
                data.as_mut_ptr(),
                data_size,
                c_kids.as_mut_ptr(),
                c_keys.as_mut_ptr(),
                1,
                &mut *decrypted_data,
                decrypt_callback,
            )
        }
    };

    if result == 0 {
        Ok(*decrypted_data)
    } else {
        Err(match result {
            100 => "invalid hex format for key id".to_owned(),
            101 => "invalid key id".to_owned(),
            102 => "invalid hex format for key".to_owned(),
            x => format!("failed to decrypt data with error code {}", x),
        })
    }
}
