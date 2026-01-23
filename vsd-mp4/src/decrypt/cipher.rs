use aes::{
    Aes128,
    cipher::{BlockDecryptMut, KeyIvInit, StreamCipher},
};

type Aes128Ctr = ctr::Ctr128BE<Aes128>;
type Aes128Cbc = cbc::Decryptor<Aes128>;

pub enum CipherMode {
    Cenc,
    Cens,
    Cbc1,
    Cbcs,
    None,
}

pub struct Cipher {
    pub mode: CipherMode,
    key: [u8; 16],
    iv: [u8; 16],
    crypt_blocks: u8,
    skip_blocks: u8,
}

impl Cipher {
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
            crypt_blocks,
            skip_blocks,
        }
    }

    pub fn set_iv(&mut self, iv: &[u8; 16]) {
        self.iv = *iv;
    }

    pub fn is_cbc_mode(&self) -> bool {
        matches!(self.mode, CipherMode::Cbc1 | CipherMode::Cbcs)
    }

    pub fn is_cbcs(&self) -> bool {
        matches!(self.mode, CipherMode::Cbcs)
    }

    pub fn process(&mut self, input: &[u8], output: &mut [u8]) {
        match self.mode {
            CipherMode::Cenc => {
                Self::apply_ctr(&self.key, &self.iv, input, output);
            }

            CipherMode::Cens => {
                Self::process_pattern(
                    input,
                    output,
                    self.crypt_blocks as usize * 16,
                    self.skip_blocks as usize * 16,
                    |inp, out| Self::apply_ctr(&self.key, &self.iv, inp, out),
                    |inp, out| out.copy_from_slice(inp),
                );
            }

            CipherMode::Cbc1 => {
                Self::apply_cbc(&self.key, &self.iv, input, output);
            }

            CipherMode::Cbcs => {
                Self::process_pattern(
                    input,
                    output,
                    self.crypt_blocks as usize * 16,
                    self.skip_blocks as usize * 16,
                    |inp, out| {
                        let blocks = (inp.len() / 16) * 16;
                        if blocks > 0 {
                            Self::apply_cbc(
                                &self.key,
                                &self.iv,
                                &inp[..blocks],
                                &mut out[..blocks],
                            );
                            self.iv.copy_from_slice(&inp[blocks - 16..blocks]);
                        }
                        if blocks < inp.len() {
                            out[blocks..inp.len()].copy_from_slice(&inp[blocks..]);
                        }
                    },
                    |inp, out| out.copy_from_slice(inp),
                );
            }

            CipherMode::None => output[..input.len()].copy_from_slice(input),
        }
    }

    fn apply_ctr(key: &[u8; 16], iv: &[u8; 16], input: &[u8], output: &mut [u8]) {
        output[..input.len()].copy_from_slice(input);
        let mut cipher = Aes128Ctr::new(key.into(), iv.into());
        cipher.apply_keystream(&mut output[..input.len()]);
    }

    fn apply_cbc(key: &[u8; 16], iv: &[u8; 16], input: &[u8], output: &mut [u8]) {
        let block_count = input.len() / 16;
        if block_count == 0 {
            return;
        }

        let encrypted_size = block_count * 16;
        output[..encrypted_size].copy_from_slice(&input[..encrypted_size]);
        Aes128Cbc::new(key.into(), iv.into())
            .decrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut output[..encrypted_size])
            .unwrap();

        if encrypted_size < input.len() {
            output[encrypted_size..input.len()].copy_from_slice(&input[encrypted_size..]);
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
