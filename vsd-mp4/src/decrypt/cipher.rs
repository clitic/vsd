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
            0x63656E63 | 0x63656E73 => CipherMode::AesCtr, // 'cenc' | 'cens'
            0x63626331 | 0x63626373 => CipherMode::AesCbc, // 'cbc1' | 'cbcs'
            _ => CipherMode::None,
        }
    }
}

type Aes128Ctr = ctr::Ctr128BE<Aes128>;

pub enum Cipher {
    Cenc {
        key: [u8; 16],
        iv: [u8; 16],
        cipher: Option<Aes128Ctr>,
    },
    Cens {
        key: [u8; 16],
        iv: [u8; 16],
        cipher: Option<Aes128Ctr>,
        crypt_blocks: u8,
        skip_blocks: u8,
    },
    Cbc1 {
        key: [u8; 16],
        iv: [u8; 16],
    },
    Cbcs {
        key: [u8; 16],
        iv: [u8; 16],
        crypt_blocks: u8,
        skip_blocks: u8,
    },
    None,
}

impl Cipher {
    pub fn new(mode: CipherMode, key: &[u8], crypt_blocks: u8, skip_blocks: u8) -> Result<Self> {
        if key.len() != 16 {
            return Err(DecryptError::InvalidKeySize(key.len()));
        }

        let key: [u8; 16] = key.try_into().unwrap();
        let has_pattern = crypt_blocks > 0 || skip_blocks > 0;

        Ok(match mode {
            CipherMode::None => Cipher::None,
            CipherMode::AesCtr if has_pattern => Cipher::Cens {
                key,
                iv: [0u8; 16],
                cipher: None,
                crypt_blocks,
                skip_blocks,
            },
            CipherMode::AesCtr => Cipher::Cenc {
                key,
                iv: [0u8; 16],
                cipher: None,
            },
            CipherMode::AesCbc if has_pattern => Cipher::Cbcs {
                key,
                iv: [0u8; 16],
                crypt_blocks,
                skip_blocks,
            },
            CipherMode::AesCbc => Cipher::Cbc1 { key, iv: [0u8; 16] },
        })
    }

    pub fn set_iv(&mut self, iv: &[u8]) -> Result<()> {
        match self {
            Cipher::None => {}
            Cipher::Cenc {
                key,
                iv: stored_iv,
                cipher,
            }
            | Cipher::Cens {
                key,
                iv: stored_iv,
                cipher,
                ..
            } => {
                *stored_iv = [0u8; 16];
                stored_iv[..iv.len().min(16)].copy_from_slice(&iv[..iv.len().min(16)]);
                *cipher = Some(Aes128Ctr::new(
                    GenericArray::from_slice(key),
                    GenericArray::from_slice(stored_iv),
                ));
            }
            Cipher::Cbc1 { iv: stored_iv, .. } | Cipher::Cbcs { iv: stored_iv, .. } => {
                if iv.len() != 16 {
                    return Err(DecryptError::InvalidIvSize {
                        expected: 16,
                        actual: iv.len(),
                    });
                }
                stored_iv.copy_from_slice(iv);
            }
        }
        Ok(())
    }

    pub fn process_buffer(&mut self, input: &[u8], output: &mut [u8]) {
        match self {
            Cipher::None => output[..input.len()].copy_from_slice(input),

            Cipher::Cenc { key, iv, cipher } => {
                Self::apply_ctr(key, iv, cipher, input, output);
            }

            Cipher::Cens {
                key,
                iv,
                cipher,
                crypt_blocks,
                skip_blocks,
            } => {
                Self::process_pattern(
                    input,
                    output,
                    *crypt_blocks as usize * 16,
                    *skip_blocks as usize * 16,
                    |inp, out| Self::apply_ctr(key, iv, cipher, inp, out),
                    |inp, out| out.copy_from_slice(inp),
                );
            }

            Cipher::Cbc1 { key, iv } => {
                Self::apply_cbc(key, iv, input, output);
            }

            Cipher::Cbcs {
                key,
                iv,
                crypt_blocks,
                skip_blocks,
            } => {
                Self::process_pattern(
                    input,
                    output,
                    *crypt_blocks as usize * 16,
                    *skip_blocks as usize * 16,
                    |inp, out| {
                        let blocks = (inp.len() / 16) * 16;
                        if blocks > 0 {
                            Self::apply_cbc(key, iv, &inp[..blocks], &mut out[..blocks]);
                            iv.copy_from_slice(&inp[blocks - 16..blocks]);
                        }
                        if blocks < inp.len() {
                            out[blocks..inp.len()].copy_from_slice(&inp[blocks..]);
                        }
                    },
                    |inp, out| out.copy_from_slice(inp),
                );
            }
        }
    }

    pub fn is_cbc_mode(&self) -> bool {
        matches!(self, Cipher::Cbc1 { .. } | Cipher::Cbcs { .. })
    }

    pub fn resets_iv_per_subsample(&self) -> bool {
        matches!(self, Cipher::Cbcs { .. })
    }

    fn apply_ctr(
        key: &[u8; 16],
        iv: &[u8; 16],
        cipher: &mut Option<Aes128Ctr>,
        input: &[u8],
        output: &mut [u8],
    ) {
        output[..input.len()].copy_from_slice(input);
        let c = cipher.get_or_insert_with(|| {
            Aes128Ctr::new(GenericArray::from_slice(key), GenericArray::from_slice(iv))
        });
        c.apply_keystream(&mut output[..input.len()]);
    }

    fn apply_cbc(key: &[u8; 16], iv: &[u8; 16], input: &[u8], output: &mut [u8]) {
        let block_count = input.len() / 16;
        if block_count == 0 {
            return;
        }

        let cipher = Aes128::new(GenericArray::from_slice(key));
        let mut prev = *iv;

        for i in 0..block_count {
            let (start, end) = (i * 16, (i + 1) * 16);
            let ciphertext: [u8; 16] = input[start..end].try_into().unwrap();
            let mut block = GenericArray::clone_from_slice(&ciphertext);
            cipher.decrypt_block(&mut block);

            for j in 0..16 {
                output[start + j] = block[j] ^ prev[j];
            }
            prev = ciphertext;
        }

        // Copy partial block
        let partial_start = block_count * 16;
        if partial_start < input.len() {
            output[partial_start..input.len()].copy_from_slice(&input[partial_start..]);
        }
    }

    fn process_pattern<F, G>(
        input: &[u8],
        output: &mut [u8],
        crypt_size: usize,
        skip_size: usize,
        mut encrypt_fn: F,
        mut copy_fn: G,
    ) where
        F: FnMut(&[u8], &mut [u8]),
        G: FnMut(&[u8], &mut [u8]),
    {
        if crypt_size == 0 && skip_size == 0 {
            encrypt_fn(input, output);
            return;
        }

        let mut offset = 0;
        while offset < input.len() {
            // Encrypt blocks
            let to_encrypt = (input.len() - offset).min(crypt_size);
            if to_encrypt > 0 {
                encrypt_fn(
                    &input[offset..offset + to_encrypt],
                    &mut output[offset..offset + to_encrypt],
                );
                offset += to_encrypt;
            }

            if offset >= input.len() {
                break;
            }

            // Skip blocks (copy as-is)
            let to_skip = (input.len() - offset).min(skip_size);
            if to_skip > 0 {
                copy_fn(
                    &input[offset..offset + to_skip],
                    &mut output[offset..offset + to_skip],
                );
                offset += to_skip;
            }
        }
    }
}
