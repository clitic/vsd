//! Single sample decrypter for CENC/CBCS.
//!
//! Handles decryption of individual samples with subsample support.

use crate::decrypt::{
    cipher::{
        CbcStreamCipher, CipherMode, CtrPatternStreamCipher, CtrStreamCipher, PatternStreamCipher,
    },
    error::{DecryptError, Result},
};

/// Single sample decrypter for CENC/CBCS encrypted samples.
///
/// Handles both full sample decryption and subsample decryption with
/// support for AES-CTR (CENC) and AES-CBC (CBCS) modes.
pub struct SingleSampleDecrypter {
    /// The cipher mode being used.
    mode: CipherMode,
    /// CTR cipher (if mode is AesCtr without pattern).
    ctr_cipher: Option<CtrStreamCipher>,
    /// CTR pattern cipher (if mode is AesCtr with pattern - CENS).
    ctr_pattern_cipher: Option<CtrPatternStreamCipher>,
    /// CBC cipher (if mode is AesCbc without pattern).
    cbc_cipher: Option<CbcStreamCipher>,
    /// Pattern cipher (if mode is AesCbc with pattern - CBCS).
    pattern_cipher: Option<PatternStreamCipher>,
    /// Whether to only process full blocks (for CBC modes).
    full_blocks_only: bool,
    /// Whether to reset IV at each subsample (for CBCS).
    reset_iv_at_each_subsample: bool,
}

impl SingleSampleDecrypter {
    /// Create a new single sample decrypter.
    ///
    /// # Arguments
    ///
    /// * `mode` - The cipher mode to use
    /// * `key` - 16-byte AES key
    /// * `crypt_byte_block` - Blocks to encrypt in pattern (0 for no pattern)
    /// * `skip_byte_block` - Blocks to skip in pattern (0 for no pattern)
    /// * `reset_iv_at_each_subsample` - Whether to reset IV per subsample (CBCS)
    pub fn new(
        mode: CipherMode,
        key: &[u8],
        crypt_byte_block: u8,
        skip_byte_block: u8,
        reset_iv_at_each_subsample: bool,
    ) -> Result<Self> {
        if key.len() != 16 {
            return Err(DecryptError::InvalidKeySize(key.len()));
        }

        let (ctr_cipher, ctr_pattern_cipher, cbc_cipher, pattern_cipher, full_blocks_only) =
            match mode {
                CipherMode::None => (None, None, None, None, false),
                CipherMode::AesCtr => {
                    if crypt_byte_block > 0 || skip_byte_block > 0 {
                        // Pattern encryption with CTR (CENS)
                        let cipher =
                            CtrPatternStreamCipher::new(key, crypt_byte_block, skip_byte_block)?;
                        (None, Some(cipher), None, None, false)
                    } else {
                        // Full CTR encryption (CENC)
                        let cipher = CtrStreamCipher::new(key, 16)?;
                        (Some(cipher), None, None, None, false)
                    }
                }
                CipherMode::AesCbc => {
                    if crypt_byte_block > 0 || skip_byte_block > 0 {
                        // Pattern encryption with CBC (CBCS)
                        let cipher =
                            PatternStreamCipher::new(key, crypt_byte_block, skip_byte_block)?;
                        (None, None, None, Some(cipher), true)
                    } else {
                        // Full CBC encryption (CBC1)
                        let cipher = CbcStreamCipher::new(key)?;
                        (None, None, Some(cipher), None, true)
                    }
                }
            };

        Ok(Self {
            mode,
            ctr_cipher,
            ctr_pattern_cipher,
            cbc_cipher,
            pattern_cipher,
            full_blocks_only,
            reset_iv_at_each_subsample,
        })
    }

    /// Decrypt sample data.
    ///
    /// # Arguments
    ///
    /// * `data_in` - Encrypted sample data
    /// * `iv` - 16-byte initialization vector
    /// * `subsample_count` - Number of subsamples (0 for full encryption)
    /// * `bytes_of_cleartext_data` - Array of cleartext byte counts per subsample
    /// * `bytes_of_encrypted_data` - Array of encrypted byte counts per subsample
    ///
    /// # Returns
    ///
    /// Decrypted sample data.
    pub fn decrypt_sample_data(
        &mut self,
        data_in: &[u8],
        iv: &[u8; 16],
        subsample_count: usize,
        bytes_of_cleartext_data: &[u16],
        bytes_of_encrypted_data: &[u32],
    ) -> Result<Vec<u8>> {
        if self.mode == CipherMode::None {
            // No encryption, just copy
            return Ok(data_in.to_vec());
        }

        let mut data_out = vec![0u8; data_in.len()];

        // Set the IV
        self.set_iv(iv)?;

        if subsample_count > 0 {
            // Subsample decryption
            self.decrypt_subsamples(
                data_in,
                &mut data_out,
                iv,
                bytes_of_cleartext_data,
                bytes_of_encrypted_data,
            )?;
        } else if self.full_blocks_only {
            // CBC mode: only decrypt full blocks
            self.decrypt_full_blocks(data_in, &mut data_out)?;
        } else {
            // CTR mode: decrypt entire sample
            self.decrypt_full_sample(data_in, &mut data_out)?;
        }

        Ok(data_out)
    }

    /// Set the IV on the cipher.
    fn set_iv(&mut self, iv: &[u8]) -> Result<()> {
        if let Some(ref mut cipher) = self.ctr_cipher {
            cipher.set_iv(iv)?;
        }
        if let Some(ref mut cipher) = self.ctr_pattern_cipher {
            cipher.set_iv(iv)?;
        }
        if let Some(ref mut cipher) = self.cbc_cipher {
            cipher.set_iv(iv)?;
        }
        if let Some(ref mut cipher) = self.pattern_cipher {
            cipher.set_iv(iv)?;
        }
        Ok(())
    }

    /// Decrypt with subsample map.
    fn decrypt_subsamples(
        &mut self,
        data_in: &[u8],
        data_out: &mut [u8],
        iv: &[u8; 16],
        bytes_of_cleartext_data: &[u16],
        bytes_of_encrypted_data: &[u32],
    ) -> Result<()> {
        let mut in_offset = 0usize;
        let mut out_offset = 0usize;

        for i in 0..bytes_of_cleartext_data.len() {
            let cleartext_size = bytes_of_cleartext_data[i] as usize;
            let encrypted_size = bytes_of_encrypted_data[i] as usize;

            // Check bounds - if subsample extends beyond data, fall back to full-block decryption
            if in_offset + cleartext_size + encrypted_size > data_in.len() {
                // For CBC modes, decrypt remaining data as full blocks
                let remaining = &data_in[in_offset..];
                let remaining_out = &mut data_out[out_offset..];
                if self.full_blocks_only && remaining.len() >= 16 {
                    // Reset IV for this fallback
                    let _ = self.set_iv(iv);
                    self.process_buffer(remaining, remaining_out)?;
                } else {
                    // CTR mode or partial block - copy as-is
                    remaining_out.copy_from_slice(remaining);
                }
                return Ok(());
            }

            // Copy cleartext portion
            if cleartext_size > 0 {
                data_out[out_offset..out_offset + cleartext_size]
                    .copy_from_slice(&data_in[in_offset..in_offset + cleartext_size]);
            }

            // Decrypt encrypted portion
            if encrypted_size > 0 {
                // Reset IV for each subsample if required (CBCS)
                if self.reset_iv_at_each_subsample {
                    self.set_iv(iv)?;
                }

                let encrypted_in = &data_in
                    [in_offset + cleartext_size..in_offset + cleartext_size + encrypted_size];
                let encrypted_out = &mut data_out
                    [out_offset + cleartext_size..out_offset + cleartext_size + encrypted_size];

                self.process_buffer(encrypted_in, encrypted_out)?;
            }

            in_offset += cleartext_size + encrypted_size;
            out_offset += cleartext_size + encrypted_size;
        }

        // Copy any leftover partial block
        if in_offset < data_in.len() {
            let remaining = data_in.len() - in_offset;
            data_out[out_offset..out_offset + remaining].copy_from_slice(&data_in[in_offset..]);
        }

        Ok(())
    }

    /// Decrypt full blocks (for CBC mode).
    fn decrypt_full_blocks(&mut self, data_in: &[u8], data_out: &mut [u8]) -> Result<()> {
        let block_count = data_in.len() / 16;

        if block_count > 0 {
            let encrypted_size = block_count * 16;
            self.process_buffer(&data_in[..encrypted_size], &mut data_out[..encrypted_size])?;

            // Copy remaining partial block (stays in clear)
            if encrypted_size < data_in.len() {
                data_out[encrypted_size..].copy_from_slice(&data_in[encrypted_size..]);
            }
        } else {
            // No full blocks, copy everything as-is
            data_out.copy_from_slice(data_in);
        }

        Ok(())
    }

    /// Decrypt full sample (for CTR mode).
    fn decrypt_full_sample(&mut self, data_in: &[u8], data_out: &mut [u8]) -> Result<()> {
        self.process_buffer(data_in, data_out)
    }

    /// Process buffer with the active cipher.
    fn process_buffer(&mut self, input: &[u8], output: &mut [u8]) -> Result<()> {
        if let Some(ref mut cipher) = self.ctr_cipher {
            cipher.process_buffer(input, output);
            return Ok(());
        }
        if let Some(ref mut cipher) = self.ctr_pattern_cipher {
            cipher.process_buffer(input, output);
            return Ok(());
        }
        if let Some(ref mut cipher) = self.cbc_cipher {
            cipher.process_buffer(input, output);
            return Ok(());
        }
        if let Some(ref mut cipher) = self.pattern_cipher {
            cipher.process_buffer(input, output);
            return Ok(());
        }

        // No cipher (shouldn't happen if mode != None)
        output[..input.len()].copy_from_slice(input);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypter_none_mode() {
        let mut decrypter =
            SingleSampleDecrypter::new(CipherMode::None, &[0u8; 16], 0, 0, false).unwrap();

        let data = b"Hello, World!";
        let iv = [0u8; 16];

        let result = decrypter
            .decrypt_sample_data(data, &iv, 0, &[], &[])
            .unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_decrypter_ctr_mode() {
        let key = [0u8; 16];
        let mut decrypter =
            SingleSampleDecrypter::new(CipherMode::AesCtr, &key, 0, 0, false).unwrap();

        let data = [0u8; 32];
        let iv = [0u8; 16];

        let result = decrypter
            .decrypt_sample_data(&data, &iv, 0, &[], &[])
            .unwrap();

        // CTR mode will produce different output
        assert_ne!(result, data);
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_decrypter_with_subsamples() {
        let key = [0u8; 16];
        let mut decrypter =
            SingleSampleDecrypter::new(CipherMode::AesCtr, &key, 0, 0, false).unwrap();

        // Data: 10 bytes clear, 16 bytes encrypted
        let data = [0u8; 26];
        let iv = [0u8; 16];

        let result = decrypter
            .decrypt_sample_data(&data, &iv, 1, &[10], &[16])
            .unwrap();

        assert_eq!(result.len(), 26);
        // First 10 bytes should be unchanged (cleartext)
        assert_eq!(&result[..10], &[0u8; 10]);
    }
}
