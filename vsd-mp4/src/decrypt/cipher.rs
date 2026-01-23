//! AES stream cipher implementations for CENC/CBCS decryption.

use crate::decrypt::error::{DecryptError, Result};
use aes::{
    Aes128,
    cipher::{BlockDecrypt, KeyInit, KeyIvInit, StreamCipher, generic_array::GenericArray},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherMode {
    None,
    AesCtr,
    AesCbc,
}

impl CipherMode {
    pub fn from_scheme_type(scheme_type: u32) -> Self {
        match scheme_type {
            0x63656E63 => CipherMode::AesCtr, // 'cenc'
            0x63656E73 => CipherMode::AesCtr, // 'cens'
            0x63626331 => CipherMode::AesCbc, // 'cbc1'
            0x63626373 => CipherMode::AesCbc, // 'cbcs'
            _ => CipherMode::None,
        }
    }

    pub fn resets_iv_per_subsample(scheme_type: u32) -> bool {
        scheme_type == 0x63626373 // 'cbcs'
    }
}

type Aes128Ctr = ctr::Ctr128BE<Aes128>;

pub enum Cipher {
    None,
    Cenc {
        key: [u8; 16],
        iv: [u8; 16],
        cipher: Option<Aes128Ctr>,
    },
    Cens {
        key: [u8; 16],
        iv: [u8; 16],
        cipher: Option<Aes128Ctr>,
        crypt_byte_block: u8,
        skip_byte_block: u8,
    },
    Cbc1 {
        key: [u8; 16],
        iv: [u8; 16],
    },
    Cbcs {
        key: [u8; 16],
        iv: [u8; 16],
        crypt_byte_block: u8,
        skip_byte_block: u8,
    },
}

impl Cipher {
    pub fn new(
        mode: CipherMode,
        key: &[u8],
        crypt_byte_block: u8,
        skip_byte_block: u8,
    ) -> Result<Self> {
        if key.len() != 16 {
            return Err(DecryptError::InvalidKeySize(key.len()));
        }

        let mut key_arr = [0u8; 16];
        key_arr.copy_from_slice(key);

        match mode {
            CipherMode::None => Ok(Cipher::None),
            CipherMode::AesCtr => {
                if crypt_byte_block > 0 || skip_byte_block > 0 {
                    Ok(Cipher::Cens {
                        key: key_arr,
                        iv: [0u8; 16],
                        cipher: None,
                        crypt_byte_block,
                        skip_byte_block,
                    })
                } else {
                    Ok(Cipher::Cenc {
                        key: key_arr,
                        iv: [0u8; 16],
                        cipher: None,
                    })
                }
            }
            CipherMode::AesCbc => {
                if crypt_byte_block > 0 || skip_byte_block > 0 {
                    Ok(Cipher::Cbcs {
                        key: key_arr,
                        iv: [0u8; 16],
                        crypt_byte_block,
                        skip_byte_block,
                    })
                } else {
                    Ok(Cipher::Cbc1 {
                        key: key_arr,
                        iv: [0u8; 16],
                    })
                }
            }
        }
    }

    pub fn set_iv(&mut self, iv: &[u8]) -> Result<()> {
        match self {
            Cipher::None => Ok(()),
            Cipher::Cenc {
                key,
                iv: stored_iv,
                cipher,
            } => {
                let iv_len = iv.len().min(16);
                *stored_iv = [0u8; 16];
                stored_iv[..iv_len].copy_from_slice(&iv[..iv_len]);

                let key_ga = GenericArray::from_slice(key);
                let nonce = GenericArray::from_slice(stored_iv);
                *cipher = Some(Aes128Ctr::new(key_ga, nonce));
                Ok(())
            }
            Cipher::Cens {
                key,
                iv: stored_iv,
                cipher,
                ..
            } => {
                let iv_len = iv.len().min(16);
                *stored_iv = [0u8; 16];
                stored_iv[..iv_len].copy_from_slice(&iv[..iv_len]);

                let key_ga = GenericArray::from_slice(key);
                let nonce = GenericArray::from_slice(stored_iv);
                *cipher = Some(Aes128Ctr::new(key_ga, nonce));
                Ok(())
            }
            Cipher::Cbc1 { iv: stored_iv, .. } | Cipher::Cbcs { iv: stored_iv, .. } => {
                if iv.len() != 16 {
                    return Err(DecryptError::InvalidIvSize {
                        expected: 16,
                        actual: iv.len(),
                    });
                }
                stored_iv.copy_from_slice(iv);
                Ok(())
            }
        }
    }

    pub fn process_buffer(&mut self, input: &[u8], output: &mut [u8]) {
        match self {
            Cipher::None => {
                output[..input.len()].copy_from_slice(input);
            }
            Cipher::Cenc { key, iv, cipher } => {
                output[..input.len()].copy_from_slice(input);
                if let Some(c) = cipher {
                    c.apply_keystream(&mut output[..input.len()]);
                } else {
                    let key_ga = GenericArray::from_slice(key);
                    let nonce = GenericArray::from_slice(iv);
                    let mut c = Aes128Ctr::new(key_ga, nonce);
                    c.apply_keystream(&mut output[..input.len()]);
                    *cipher = Some(c);
                }
            }
            Cipher::Cens {
                key,
                iv,
                cipher,
                crypt_byte_block,
                skip_byte_block,
            } => {
                let crypt_size = *crypt_byte_block as usize * 16;
                let skip_size = *skip_byte_block as usize * 16;

                if crypt_size == 0 && skip_size == 0 {
                    output[..input.len()].copy_from_slice(input);
                    if let Some(c) = cipher {
                        c.apply_keystream(&mut output[..input.len()]);
                    } else {
                        let key_ga = GenericArray::from_slice(key);
                        let nonce = GenericArray::from_slice(iv);
                        let mut c = Aes128Ctr::new(key_ga, nonce);
                        c.apply_keystream(&mut output[..input.len()]);
                        *cipher = Some(c);
                    }
                    return;
                }

                let mut offset = 0;
                while offset < input.len() {
                    let remaining = input.len() - offset;

                    let to_encrypt = remaining.min(crypt_size);
                    if to_encrypt > 0 {
                        output[offset..offset + to_encrypt]
                            .copy_from_slice(&input[offset..offset + to_encrypt]);
                        if let Some(c) = cipher {
                            c.apply_keystream(&mut output[offset..offset + to_encrypt]);
                        } else {
                            let key_ga = GenericArray::from_slice(key);
                            let nonce = GenericArray::from_slice(iv);
                            let mut c = Aes128Ctr::new(key_ga, nonce);
                            c.apply_keystream(&mut output[offset..offset + to_encrypt]);
                            *cipher = Some(c);
                        }
                        offset += to_encrypt;
                    }

                    if offset >= input.len() {
                        break;
                    }

                    let remaining = input.len() - offset;
                    let to_skip = remaining.min(skip_size);
                    if to_skip > 0 {
                        output[offset..offset + to_skip]
                            .copy_from_slice(&input[offset..offset + to_skip]);
                        offset += to_skip;
                    }
                }
            }
            Cipher::Cbc1 { key, iv } => {
                Self::process_cbc(key, iv, input, output);
            }
            Cipher::Cbcs {
                key,
                iv,
                crypt_byte_block,
                skip_byte_block,
            } => {
                let crypt_size = *crypt_byte_block as usize * 16;
                let skip_size = *skip_byte_block as usize * 16;

                if crypt_size == 0 && skip_size == 0 {
                    Self::process_cbc(key, iv, input, output);
                    return;
                }

                let mut offset = 0;
                while offset < input.len() {
                    let remaining = input.len() - offset;

                    let to_encrypt = remaining.min(crypt_size);
                    if to_encrypt >= 16 {
                        let blocks_to_encrypt = (to_encrypt / 16) * 16;
                        Self::process_cbc(
                            key,
                            iv,
                            &input[offset..offset + blocks_to_encrypt],
                            &mut output[offset..offset + blocks_to_encrypt],
                        );

                        if blocks_to_encrypt >= 16 {
                            let last_block_start = offset + blocks_to_encrypt - 16;
                            iv.copy_from_slice(&input[last_block_start..last_block_start + 16]);
                        }

                        offset += blocks_to_encrypt;
                    } else if to_encrypt > 0 {
                        output[offset..offset + to_encrypt]
                            .copy_from_slice(&input[offset..offset + to_encrypt]);
                        offset += to_encrypt;
                    }

                    let remaining = input.len() - offset;
                    let to_skip = remaining.min(skip_size);
                    if to_skip > 0 {
                        output[offset..offset + to_skip]
                            .copy_from_slice(&input[offset..offset + to_skip]);
                        offset += to_skip;
                    }
                }
            }
        }
    }

    pub fn is_cbc_mode(&self) -> bool {
        matches!(self, Cipher::Cbc1 { .. } | Cipher::Cbcs { .. })
    }

    fn process_cbc(key: &[u8; 16], iv: &[u8; 16], input: &[u8], output: &mut [u8]) {
        let block_count = input.len() / 16;
        let encrypted_size = block_count * 16;

        if encrypted_size > 0 {
            let cipher = Aes128::new(GenericArray::from_slice(key));
            let mut prev_block = *iv;

            for i in 0..block_count {
                let start = i * 16;
                let end = start + 16;

                let ciphertext: [u8; 16] = input[start..end].try_into().unwrap();
                let mut block = GenericArray::clone_from_slice(&ciphertext);
                cipher.decrypt_block(&mut block);

                for j in 0..16 {
                    output[start + j] = block[j] ^ prev_block[j];
                }

                prev_block = ciphertext;
            }
        }

        let partial = input.len() % 16;
        if partial > 0 {
            output[encrypted_size..encrypted_size + partial]
                .copy_from_slice(&input[encrypted_size..]);
        }
    }
}
