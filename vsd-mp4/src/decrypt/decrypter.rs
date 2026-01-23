use crate::decrypt::{
    cipher::{Cbc1Cipher, CbcsCipher, CencCipher, CensCipher, Cipher, CipherMode},
    error::{DecryptError, Result},
};

pub struct SingleSampleDecrypter {
    cipher: Cipher,
    full_blocks_only: bool,
    reset_iv_at_each_subsample: bool,
}

impl SingleSampleDecrypter {
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

        let (cipher, full_blocks_only) = match mode {
            CipherMode::None => (Cipher::None, false),
            CipherMode::AesCtr => {
                if crypt_byte_block > 0 || skip_byte_block > 0 {
                    let c = CensCipher::new(key, crypt_byte_block, skip_byte_block)?;
                    (Cipher::Cens(c), false)
                } else {
                    let c = CencCipher::new(key, 16)?;
                    (Cipher::Cenc(c), false)
                }
            }
            CipherMode::AesCbc => {
                if crypt_byte_block > 0 || skip_byte_block > 0 {
                    let c = CbcsCipher::new(key, crypt_byte_block, skip_byte_block)?;
                    (Cipher::Cbcs(c), true)
                } else {
                    let c = Cbc1Cipher::new(key)?;
                    (Cipher::Cbc1(c), true)
                }
            }
        };

        Ok(Self {
            cipher,
            full_blocks_only,
            reset_iv_at_each_subsample,
        })
    }

    pub fn decrypt_sample_data(
        &mut self,
        data_in: &[u8],
        iv: &[u8; 16],
        subsample_count: usize,
        bytes_of_cleartext_data: &[u16],
        bytes_of_encrypted_data: &[u32],
    ) -> Result<Vec<u8>> {
        if matches!(self.cipher, Cipher::None) {
            return Ok(data_in.to_vec());
        }

        let mut data_out = vec![0u8; data_in.len()];
        self.set_iv(iv)?;

        if subsample_count > 0 {
            self.decrypt_subsamples(
                data_in,
                &mut data_out,
                iv,
                bytes_of_cleartext_data,
                bytes_of_encrypted_data,
            )?;
        } else if self.full_blocks_only {
            self.decrypt_full_blocks(data_in, &mut data_out)?;
        } else {
            self.decrypt_full_sample(data_in, &mut data_out)?;
        }

        Ok(data_out)
    }

    fn set_iv(&mut self, iv: &[u8]) -> Result<()> {
        match &mut self.cipher {
            Cipher::None => {}
            Cipher::Cenc(c) => c.set_iv(iv)?,
            Cipher::Cens(c) => c.set_iv(iv)?,
            Cipher::Cbc1(c) => c.set_iv(iv)?,
            Cipher::Cbcs(c) => c.set_iv(iv)?,
        }
        Ok(())
    }

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

            if in_offset + cleartext_size + encrypted_size > data_in.len() {
                let remaining = &data_in[in_offset..];
                let remaining_out = &mut data_out[out_offset..];
                if self.full_blocks_only && remaining.len() >= 16 {
                    let _ = self.set_iv(iv);
                    self.process_buffer(remaining, remaining_out)?;
                } else {
                    remaining_out.copy_from_slice(remaining);
                }
                return Ok(());
            }

            if cleartext_size > 0 {
                data_out[out_offset..out_offset + cleartext_size]
                    .copy_from_slice(&data_in[in_offset..in_offset + cleartext_size]);
            }

            if encrypted_size > 0 {
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

        if in_offset < data_in.len() {
            let remaining = data_in.len() - in_offset;
            data_out[out_offset..out_offset + remaining].copy_from_slice(&data_in[in_offset..]);
        }

        Ok(())
    }

    fn decrypt_full_blocks(&mut self, data_in: &[u8], data_out: &mut [u8]) -> Result<()> {
        let block_count = data_in.len() / 16;

        if block_count > 0 {
            let encrypted_size = block_count * 16;
            self.process_buffer(&data_in[..encrypted_size], &mut data_out[..encrypted_size])?;

            if encrypted_size < data_in.len() {
                data_out[encrypted_size..].copy_from_slice(&data_in[encrypted_size..]);
            }
        } else {
            data_out.copy_from_slice(data_in);
        }

        Ok(())
    }

    fn decrypt_full_sample(&mut self, data_in: &[u8], data_out: &mut [u8]) -> Result<()> {
        self.process_buffer(data_in, data_out)
    }

    fn process_buffer(&mut self, input: &[u8], output: &mut [u8]) -> Result<()> {
        match &mut self.cipher {
            Cipher::None => output[..input.len()].copy_from_slice(input),
            Cipher::Cenc(c) => c.process_buffer(input, output),
            Cipher::Cens(c) => c.process_buffer(input, output),
            Cipher::Cbc1(c) => c.process_buffer(input, output),
            Cipher::Cbcs(c) => c.process_buffer(input, output),
        }
        Ok(())
    }
}
