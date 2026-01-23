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
            );
        } else if let CipherMode::Cbc1 | CipherMode::Cbcs = self.0.mode {
            self.decrypt_full_blocks(data_in, &mut data_out);
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
    ) {
        let mut offset = 0;

        for (&clear, &enc) in bytes_of_cleartext_data.iter().zip(bytes_of_encrypted_data) {
            let clear_size = clear as usize;
            let enc_size = enc as usize;

            if offset + clear_size + enc_size > data_in.len() {
                let remaining = &data_in[offset..];
                if let CipherMode::Cbc1 | CipherMode::Cbcs = self.0.mode {
                    self.0.set_iv(iv);
                    self.0.process(remaining, &mut data_out[offset..]);
                } else {
                    data_out[offset..].copy_from_slice(remaining);
                }
                return;
            }

            if clear_size > 0 {
                data_out[offset..offset + clear_size]
                    .copy_from_slice(&data_in[offset..offset + clear_size]);
            }

            if enc_size > 0 {
                if let CipherMode::Cbcs = self.0.mode {
                    self.0.set_iv(iv);
                }
                let start = offset + clear_size;
                self.0.process(
                    &data_in[start..start + enc_size],
                    &mut data_out[start..start + enc_size],
                );
            }

            offset += clear_size + enc_size;
        }

        if offset < data_in.len() {
            data_out[offset..].copy_from_slice(&data_in[offset..]);
        }
    }

    fn decrypt_full_blocks(&mut self, data_in: &[u8], data_out: &mut [u8]) {
        let blocks = (data_in.len() / 16) * 16;
        if blocks > 0 {
            self.0.process(&data_in[..blocks], &mut data_out[..blocks]);
        }
        if blocks < data_in.len() {
            data_out[blocks..].copy_from_slice(&data_in[blocks..]);
        }
    }
}
