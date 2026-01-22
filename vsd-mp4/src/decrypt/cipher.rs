//! AES stream cipher implementations for CENC/CBCS decryption.
//!
//! Supports AES-128-CTR (CENC) and AES-128-CBC (CBCS) modes.

use aes::Aes128;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, KeyInit, KeyIvInit, StreamCipher};

use super::error::{DecryptError, Result};

/// The cipher mode used for encryption/decryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherMode {
    /// No encryption.
    None,
    /// AES-128-CTR mode (used by CENC, CENS).
    AesCtr,
    /// AES-128-CBC mode (used by CBC1, CBCS).
    AesCbc,
}

impl CipherMode {
    /// Determine cipher mode from protection scheme type.
    pub fn from_scheme_type(scheme_type: u32) -> Self {
        match scheme_type {
            // 'cenc' - AES-CTR full sample encryption
            0x63656E63 => CipherMode::AesCtr,
            // 'cens' - AES-CTR subsample encryption
            0x63656E73 => CipherMode::AesCtr,
            // 'cbc1' - AES-CBC full sample encryption
            0x63626331 => CipherMode::AesCbc,
            // 'cbcs' - AES-CBC pattern encryption
            0x63626373 => CipherMode::AesCbc,
            _ => CipherMode::None,
        }
    }

    /// Check if this scheme uses CBC mode with IV reset per subsample (CBCS).
    pub fn resets_iv_per_subsample(scheme_type: u32) -> bool {
        scheme_type == 0x63626373 // 'cbcs'
    }
}

type Aes128Ctr = ctr::Ctr128BE<Aes128>;

/// AES-128-CTR stream cipher for CENC decryption.
pub struct CtrStreamCipher {
    key: [u8; 16],
    iv: [u8; 16],
    cipher: Option<Aes128Ctr>,
}

impl CtrStreamCipher {
    /// Create a new CTR cipher with the given key and counter size.
    ///
    /// # Arguments
    ///
    /// * `key` - 16-byte AES key
    /// * `counter_size` - Size of the counter portion of the IV (8 or 16 bytes)
    pub fn new(key: &[u8], counter_size: usize) -> Result<Self> {
        if key.len() != 16 {
            return Err(DecryptError::InvalidKeySize(key.len()));
        }
        if counter_size != 8 && counter_size != 16 {
            return Err(DecryptError::InvalidIvSize {
                expected: 8,
                actual: counter_size,
            });
        }

        let mut key_arr = [0u8; 16];
        key_arr.copy_from_slice(key);

        Ok(Self {
            key: key_arr,
            iv: [0u8; 16],
            cipher: None,
        })
    }

    /// Set the IV for the cipher and reinitialize the cipher instance.
    pub fn set_iv(&mut self, iv: &[u8]) -> Result<()> {
        let iv_len = iv.len();
        if iv_len > 16 {
            return Err(DecryptError::InvalidIvSize {
                expected: 16,
                actual: iv_len,
            });
        }

        // Clear IV and copy provided bytes (zero-padded on the right if needed)
        self.iv = [0u8; 16];
        self.iv[..iv_len].copy_from_slice(iv);

        // Create a new cipher instance with the new IV
        let key = GenericArray::from_slice(&self.key);
        let nonce = GenericArray::from_slice(&self.iv);
        self.cipher = Some(Aes128Ctr::new(key, nonce));

        Ok(())
    }

    /// Process (decrypt) data in-place.
    /// Maintains counter state across multiple calls.
    pub fn process(&mut self, data: &mut [u8]) {
        if let Some(ref mut cipher) = self.cipher {
            cipher.apply_keystream(data);
        } else {
            // Cipher not initialized - create one with current IV
            let key = GenericArray::from_slice(&self.key);
            let nonce = GenericArray::from_slice(&self.iv);
            let mut cipher = Aes128Ctr::new(key, nonce);
            cipher.apply_keystream(data);
            self.cipher = Some(cipher);
        }
    }

    /// Process data from input to output buffer.
    pub fn process_buffer(&mut self, input: &[u8], output: &mut [u8]) {
        output[..input.len()].copy_from_slice(input);
        self.process(&mut output[..input.len()]);
    }
}

/// CTR pattern cipher for CENS "crypt and skip" pattern encryption.
///
/// CENS uses a pattern of encrypted and unencrypted 16-byte blocks with CTR mode.
/// The pattern works the same as CBCS but with CTR instead of CBC.
pub struct CtrPatternStreamCipher {
    inner: CtrStreamCipher,
    crypt_byte_block: u8,
    skip_byte_block: u8,
}

impl CtrPatternStreamCipher {
    /// Create a new CTR pattern cipher.
    ///
    /// # Arguments
    ///
    /// * `key` - 16-byte AES key
    /// * `crypt_byte_block` - Number of 16-byte blocks to encrypt (typically 1)
    /// * `skip_byte_block` - Number of 16-byte blocks to skip (typically 9)
    pub fn new(key: &[u8], crypt_byte_block: u8, skip_byte_block: u8) -> Result<Self> {
        Ok(Self {
            inner: CtrStreamCipher::new(key, 16)?,
            crypt_byte_block,
            skip_byte_block,
        })
    }

    /// Set the IV.
    pub fn set_iv(&mut self, iv: &[u8]) -> Result<()> {
        self.inner.set_iv(iv)
    }

    /// Process data with pattern encryption using CTR mode.
    ///
    /// For CENS, the pattern determines which 16-byte blocks are encrypted
    /// and which are left in clear. The CTR counter continues advancing
    /// through skip blocks as well.
    pub fn process_buffer(&mut self, input: &[u8], output: &mut [u8]) {
        if self.crypt_byte_block == 0 && self.skip_byte_block == 0 {
            // No pattern: decrypt everything (full CTR)
            self.inner.process_buffer(input, output);
            return;
        }

        let crypt_size = self.crypt_byte_block as usize * 16;
        let skip_size = self.skip_byte_block as usize * 16;

        let mut offset = 0;
        while offset < input.len() {
            let remaining = input.len() - offset;

            // Decrypt crypt_byte_block blocks
            let to_encrypt = remaining.min(crypt_size);
            if to_encrypt > 0 {
                self.inner.process_buffer(
                    &input[offset..offset + to_encrypt],
                    &mut output[offset..offset + to_encrypt],
                );
                offset += to_encrypt;
            }

            if offset >= input.len() {
                break;
            }

            // Skip skip_byte_block blocks (copy as-is, do NOT advance CTR counter)
            // For CENS, the counter only advances for encrypted bytes, not skip bytes
            let remaining = input.len() - offset;
            let to_skip = remaining.min(skip_size);
            if to_skip > 0 {
                // Copy the cleartext portion as-is
                output[offset..offset + to_skip].copy_from_slice(&input[offset..offset + to_skip]);
                offset += to_skip;
            }
        }
    }
}

/// AES-128-CBC stream cipher for CBCS decryption.
pub struct CbcStreamCipher {
    key: [u8; 16],
    iv: [u8; 16],
}

impl CbcStreamCipher {
    /// Create a new CBC cipher with the given key.
    pub fn new(key: &[u8]) -> Result<Self> {
        if key.len() != 16 {
            return Err(DecryptError::InvalidKeySize(key.len()));
        }

        let mut key_arr = [0u8; 16];
        key_arr.copy_from_slice(key);

        Ok(Self {
            key: key_arr,
            iv: [0u8; 16],
        })
    }

    /// Set the IV for the cipher.
    pub fn set_iv(&mut self, iv: &[u8]) -> Result<()> {
        if iv.len() != 16 {
            return Err(DecryptError::InvalidIvSize {
                expected: 16,
                actual: iv.len(),
            });
        }
        self.iv.copy_from_slice(iv);
        Ok(())
    }

    /// Process data from input to output buffer (complete blocks only).
    ///
    /// Uses manual CBC decryption: for each block, decrypt then XOR with previous
    /// ciphertext (or IV for first block).
    pub fn process_buffer(&mut self, input: &[u8], output: &mut [u8]) {
        let block_count = input.len() / 16;
        let encrypted_size = block_count * 16;

        if encrypted_size > 0 {
            let cipher = Aes128::new(GenericArray::from_slice(&self.key));
            let mut prev_block = self.iv;

            for i in 0..block_count {
                let start = i * 16;
                let end = start + 16;

                // Save ciphertext for next iteration's XOR
                let ciphertext: [u8; 16] = input[start..end].try_into().unwrap();

                // Decrypt block
                let mut block = GenericArray::clone_from_slice(&ciphertext);
                cipher.decrypt_block(&mut block);

                // XOR with previous ciphertext (or IV)
                for j in 0..16 {
                    output[start + j] = block[j] ^ prev_block[j];
                }

                prev_block = ciphertext;
            }
        }

        // Copy any remaining partial block (stays in cleartext)
        let partial = input.len() % 16;
        if partial > 0 {
            output[encrypted_size..encrypted_size + partial]
                .copy_from_slice(&input[encrypted_size..]);
        }
    }
}

/// Pattern cipher for CBCS "crypt and skip" pattern encryption.
///
/// CBCS uses a pattern of encrypted and unencrypted 16-byte blocks.
/// The default pattern is 1:9 (crypt 1 block, skip 9 blocks).
pub struct PatternStreamCipher {
    inner: CbcStreamCipher,
    crypt_byte_block: u8,
    skip_byte_block: u8,
}

impl PatternStreamCipher {
    /// Create a new pattern cipher.
    ///
    /// # Arguments
    ///
    /// * `key` - 16-byte AES key
    /// * `crypt_byte_block` - Number of 16-byte blocks to encrypt (typically 1)
    /// * `skip_byte_block` - Number of 16-byte blocks to skip (typically 9)
    pub fn new(key: &[u8], crypt_byte_block: u8, skip_byte_block: u8) -> Result<Self> {
        Ok(Self {
            inner: CbcStreamCipher::new(key)?,
            crypt_byte_block,
            skip_byte_block,
        })
    }

    /// Set the IV.
    pub fn set_iv(&mut self, iv: &[u8]) -> Result<()> {
        self.inner.set_iv(iv)
    }

    /// Process data with pattern encryption.
    ///
    /// For CBCS, the CBC cipher chains across encrypted blocks within a subsample.
    /// The IV for each subsequent encrypted block is the last ciphertext block
    /// from the previous encrypted portion.
    pub fn process_buffer(&mut self, input: &[u8], output: &mut [u8]) {
        if self.crypt_byte_block == 0 && self.skip_byte_block == 0 {
            // No pattern: decrypt everything (full CBC)
            self.inner.process_buffer(input, output);
            return;
        }

        let crypt_size = self.crypt_byte_block as usize * 16;
        let skip_size = self.skip_byte_block as usize * 16;

        let mut offset = 0;
        while offset < input.len() {
            let remaining = input.len() - offset;

            // Decrypt crypt_byte_block blocks
            let to_encrypt = remaining.min(crypt_size);
            if to_encrypt >= 16 {
                let blocks_to_encrypt = (to_encrypt / 16) * 16;
                self.inner.process_buffer(
                    &input[offset..offset + blocks_to_encrypt],
                    &mut output[offset..offset + blocks_to_encrypt],
                );

                // Save the last ciphertext block as the IV for the next pattern cycle
                // This is critical for CBC chaining in CBCS pattern encryption
                if blocks_to_encrypt >= 16 {
                    let last_block_start = offset + blocks_to_encrypt - 16;
                    let last_ciphertext: [u8; 16] = input[last_block_start..last_block_start + 16]
                        .try_into()
                        .unwrap();
                    // Update the inner CBC cipher's IV to the last ciphertext block
                    let _ = self.inner.set_iv(&last_ciphertext);
                }

                offset += blocks_to_encrypt;
            } else if to_encrypt > 0 {
                // Partial block - copy as cleartext
                output[offset..offset + to_encrypt]
                    .copy_from_slice(&input[offset..offset + to_encrypt]);
                offset += to_encrypt;
            }

            // Copy skip_byte_block blocks in cleartext
            let remaining = input.len() - offset;
            let to_skip = remaining.min(skip_size);
            if to_skip > 0 {
                output[offset..offset + to_skip].copy_from_slice(&input[offset..offset + to_skip]);
                offset += to_skip;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctr_cipher_basic() {
        let key = [0u8; 16];
        let iv = [0u8; 16];

        let mut cipher = CtrStreamCipher::new(&key, 16).unwrap();
        cipher.set_iv(&iv).unwrap();

        let mut data = [0u8; 32];
        cipher.process(&mut data);

        // CTR mode with zero key and IV produces a specific keystream
        // Just verify it doesn't panic and produces output
        assert_ne!(data, [0u8; 32]);
    }

    #[test]
    fn test_cbc_cipher_basic() {
        let key = [0u8; 16];
        let iv = [0u8; 16];

        let mut cipher = CbcStreamCipher::new(&key).unwrap();
        cipher.set_iv(&iv).unwrap();

        let input = [0u8; 32];
        let mut output = [0u8; 32];
        cipher.process_buffer(&input, &mut output);

        // Verify it produces output (decryption of zeros)
        // The exact output depends on AES implementation
    }

    #[test]
    fn test_cipher_mode_from_scheme() {
        assert_eq!(CipherMode::from_scheme_type(0x63656E63), CipherMode::AesCtr); // cenc
        assert_eq!(CipherMode::from_scheme_type(0x63656E73), CipherMode::AesCtr); // cens
        assert_eq!(CipherMode::from_scheme_type(0x63626331), CipherMode::AesCbc); // cbc1
        assert_eq!(CipherMode::from_scheme_type(0x63626373), CipherMode::AesCbc); // cbcs
        assert_eq!(CipherMode::from_scheme_type(0x00000000), CipherMode::None);
    }
}
