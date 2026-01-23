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
        let crypt_size = self.crypt_blocks as usize * 16;
        let skip_size = self.skip_blocks as usize * 16;

        match self.mode {
            CipherMode::Cenc => Self::process_ctr(&self.key, &self.iv, input, output),
            CipherMode::Cens => self.process_cens_pattern(input, output, crypt_size, skip_size),
            CipherMode::Cbc1 => Self::process_cbc(&self.key, &self.iv, input, output),
            CipherMode::Cbcs => self.process_cbcs_pattern(input, output, crypt_size, skip_size),
            CipherMode::None => output[..input.len()].copy_from_slice(input),
        }
    }

    fn process_ctr(key: &[u8; 16], iv: &[u8; 16], input: &[u8], output: &mut [u8]) {
        output[..input.len()].copy_from_slice(input);
        Aes128Ctr::new(key.into(), iv.into()).apply_keystream(&mut output[..input.len()]);
    }

    fn process_cens_pattern(
        &self,
        input: &[u8],
        output: &mut [u8],
        crypt_size: usize,
        skip_size: usize,
    ) {
        if crypt_size == 0 && skip_size == 0 {
            Self::process_ctr(&self.key, &self.iv, input, output);
            return;
        }

        let mut cipher = Aes128Ctr::new((&self.key).into(), (&self.iv).into());
        let mut offset = 0;

        while offset < input.len() {
            let to_encrypt = (input.len() - offset).min(crypt_size);
            if to_encrypt > 0 {
                output[offset..offset + to_encrypt]
                    .copy_from_slice(&input[offset..offset + to_encrypt]);
                cipher.apply_keystream(&mut output[offset..offset + to_encrypt]);
                offset += to_encrypt;
            }

            if offset >= input.len() {
                break;
            }

            let to_skip = (input.len() - offset).min(skip_size);
            output[offset..offset + to_skip].copy_from_slice(&input[offset..offset + to_skip]);
            offset += to_skip;
        }
    }

    fn process_cbc(key: &[u8; 16], iv: &[u8; 16], input: &[u8], output: &mut [u8]) {
        let blocks = (input.len() / 16) * 16;
        if blocks == 0 {
            return;
        }

        output[..blocks].copy_from_slice(&input[..blocks]);
        Aes128Cbc::new(key.into(), iv.into())
            .decrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut output[..blocks])
            .unwrap();

        if blocks < input.len() {
            output[blocks..input.len()].copy_from_slice(&input[blocks..]);
        }
    }

    fn process_cbcs_pattern(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        crypt_size: usize,
        skip_size: usize,
    ) {
        if crypt_size == 0 && skip_size == 0 {
            Self::process_cbc(&self.key, &self.iv, input, output);
            return;
        }

        let mut offset = 0;
        while offset < input.len() {
            let to_encrypt = (input.len() - offset).min(crypt_size);
            let blocks = (to_encrypt / 16) * 16;
            if blocks > 0 {
                Self::process_cbc(
                    &self.key,
                    &self.iv,
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

            let to_skip = (input.len() - offset).min(skip_size);
            output[offset..offset + to_skip].copy_from_slice(&input[offset..offset + to_skip]);
            offset += to_skip;
        }
    }
}
