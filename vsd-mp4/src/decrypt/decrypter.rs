use crate::decrypt::{
    cipher::{Cipher, CipherMode},
    error::Result,
};

pub struct SingleSampleDecrypter(Cipher);

impl SingleSampleDecrypter {
    pub fn new(scheme_type: u32, key: &[u8; 16], crypt_blocks: u8, skip_blocks: u8) -> Self {
        Self(Cipher::new(scheme_type, key, crypt_blocks, skip_blocks))
    }

    pub fn decrypt_sample_data(
        &mut self,
        data_in: &[u8],
        iv: &[u8; 16],
        subsample_count: usize,
        bytes_of_cleartext_data: &[u16],
        bytes_of_encrypted_data: &[u32],
    ) -> Result<Vec<u8>> {
        if let CipherMode::None = self.0.mode {
            return Ok(data_in.to_vec());
        }

        let mut data_out = vec![0u8; data_in.len()];
        self.0.set_iv(iv);

        if subsample_count > 0 {
            self.decrypt_subsamples(
                data_in,
                &mut data_out,
                iv,
                bytes_of_cleartext_data,
                bytes_of_encrypted_data,
            )?;
        } else if self.0.is_cbc_mode() {
            self.decrypt_full_blocks(data_in, &mut data_out)?;
        } else {
            self.0.process(data_in, &mut data_out);
        }

        Ok(data_out)
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
                if self.0.is_cbc_mode() && remaining.len() >= 16 {
                    let _ = self.0.set_iv(iv);
                    self.0.process(remaining, remaining_out);
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
                if self.0.is_cbcs() {
                    self.0.set_iv(iv);
                }

                let encrypted_in = &data_in
                    [in_offset + cleartext_size..in_offset + cleartext_size + encrypted_size];
                let encrypted_out = &mut data_out
                    [out_offset + cleartext_size..out_offset + cleartext_size + encrypted_size];

                self.0.process(encrypted_in, encrypted_out);
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
            self.0
                .process(&data_in[..encrypted_size], &mut data_out[..encrypted_size]);

            if encrypted_size < data_in.len() {
                data_out[encrypted_size..].copy_from_slice(&data_in[encrypted_size..]);
            }
        } else {
            data_out.copy_from_slice(data_in);
        }

        Ok(())
    }
}
