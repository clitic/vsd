use aes::{
    Aes128,
    cipher::{BlockDecryptMut, KeyIvInit, StreamCipher},
};

type Aes128Ctr = ctr::Ctr128BE<Aes128>;
type Aes128Cbc = cbc::Decryptor<Aes128>;

enum CipherMode {
    Cenc,
    Cens,
    Cbc1,
    Cbcs,
    None,
}

pub struct Decrypter {
    mode: CipherMode,
    key: [u8; 16],
    iv: [u8; 16],
    crypt_size: usize,
    skip_size: usize,
}

impl Decrypter {
    pub fn new(scheme_type: u32, key: &[u8; 16], crypt_blocks: u8, skip_blocks: u8) -> Self {
        Self {
            mode: match scheme_type {
                0x63656E63 => CipherMode::Cenc,
                0x63656E73 => CipherMode::Cens,
                0x63626331 => CipherMode::Cbc1,
                0x63626373 => CipherMode::Cbcs,
                _ => CipherMode::None,
            },
            key: *key,
            iv: [0u8; 16],
            crypt_size: crypt_blocks as usize * 16,
            skip_size: skip_blocks as usize * 16,
        }
    }

    fn process(&mut self, input: &[u8], output: &mut [u8]) {
        match self.mode {
            CipherMode::Cenc => self.process_ctr(input, output),
            CipherMode::Cens => self.process_cens_pattern(input, output),
            CipherMode::Cbc1 => self.process_cbc(input, output),
            CipherMode::Cbcs => self.process_cbcs_pattern(input, output),
            CipherMode::None => output[..input.len()].copy_from_slice(input),
        }
    }

    fn process_ctr(&self, input: &[u8], output: &mut [u8]) {
        output[..input.len()].copy_from_slice(input);
        Aes128Ctr::new((&self.key).into(), (&self.iv).into())
            .apply_keystream(&mut output[..input.len()]);
    }

    fn process_cens_pattern(&self, input: &[u8], output: &mut [u8]) {
        if self.crypt_size == 0 && self.skip_size == 0 {
            self.process_ctr(input, output);
            return;
        }

        let mut cipher = Aes128Ctr::new((&self.key).into(), (&self.iv).into());
        let mut offset = 0;

        while offset < input.len() {
            let to_encrypt = (input.len() - offset).min(self.crypt_size);
            if to_encrypt > 0 {
                output[offset..offset + to_encrypt]
                    .copy_from_slice(&input[offset..offset + to_encrypt]);
                cipher.apply_keystream(&mut output[offset..offset + to_encrypt]);
                offset += to_encrypt;
            }

            if offset >= input.len() {
                break;
            }

            let to_skip = (input.len() - offset).min(self.skip_size);
            output[offset..offset + to_skip].copy_from_slice(&input[offset..offset + to_skip]);
            offset += to_skip;
        }
    }

    fn process_cbc(&self, input: &[u8], output: &mut [u8]) {
        let blocks = (input.len() / 16) * 16;
        if blocks == 0 {
            return;
        }

        output[..blocks].copy_from_slice(&input[..blocks]);
        Aes128Cbc::new((&self.key).into(), (&self.iv).into())
            .decrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut output[..blocks])
            .unwrap();

        if blocks < input.len() {
            output[blocks..input.len()].copy_from_slice(&input[blocks..]);
        }
    }

    fn process_cbcs_pattern(&mut self, input: &[u8], output: &mut [u8]) {
        if self.crypt_size == 0 && self.skip_size == 0 {
            self.process_cbc(input, output);
            return;
        }

        let mut offset = 0;
        while offset < input.len() {
            let to_encrypt = (input.len() - offset).min(self.crypt_size);
            let blocks = (to_encrypt / 16) * 16;
            if blocks > 0 {
                self.process_cbc(
                    &input[offset..offset + blocks],
                    &mut output[offset..offset + blocks],
                );
                self.iv
                    .copy_from_slice(&input[offset + blocks - 16..offset + blocks]);
            }
            if blocks < to_encrypt {
                output[offset + blocks..offset + to_encrypt]
                    .copy_from_slice(&input[offset + blocks..offset + to_encrypt]);
            }
            offset += to_encrypt;

            if offset >= input.len() {
                break;
            }

            let to_skip = (input.len() - offset).min(self.skip_size);
            output[offset..offset + to_skip].copy_from_slice(&input[offset..offset + to_skip]);
            offset += to_skip;
        }
    }

    pub fn decrypt_sample(
        &mut self,
        input: &[u8],
        iv: &[u8; 16],
        subsample_count: usize,
        bytes_of_cleartext_data: &[u16],
        bytes_of_encrypted_data: &[u32],
    ) -> Vec<u8> {
        if let CipherMode::None = self.mode {
            return input.to_vec();
        }

        let mut output = vec![0u8; input.len()];
        self.iv = *iv;

        if subsample_count > 0 {
            self.decrypt_subsamples(
                input,
                &mut output,
                iv,
                bytes_of_cleartext_data,
                bytes_of_encrypted_data,
            );
        } else if let CipherMode::Cbc1 | CipherMode::Cbcs = self.mode {
            self.decrypt_full_blocks(input, &mut output);
        } else {
            self.process(input, &mut output);
        }

        output
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
                if let CipherMode::Cbc1 | CipherMode::Cbcs = self.mode {
                    self.iv = *iv;
                    self.process(remaining, &mut data_out[offset..]);
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
                if let CipherMode::Cbcs = self.mode {
                    self.iv = *iv;
                }
                let start = offset + clear_size;
                self.process(
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
            self.process(&data_in[..blocks], &mut data_out[..blocks]);
        }
        if blocks < data_in.len() {
            data_out[blocks..].copy_from_slice(&data_in[blocks..]);
        }
    }
}
