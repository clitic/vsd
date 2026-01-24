use cipher::{BlockDecryptMut, KeyIvInit};

type Aes128Cbc = cbc::Decryptor<aes::Aes128>;

#[derive(Clone)]
pub struct HlsAes128Decrypter {
    key: [u8; 16],
    iv: [u8; 16],
}

impl HlsAes128Decrypter {
    pub fn new(key: &[u8; 16], iv: &[u8; 16]) -> Self {
        Self { key: *key, iv: *iv }
    }

    pub fn increment_iv(&mut self) {
        self.iv = (u128::from_be_bytes(self.iv) + 1).to_be_bytes();
    }

    pub fn decrypt(&self, mut input: Vec<u8>) -> Vec<u8> {
        let slice_len = {
            let slice = Aes128Cbc::new((&self.key).into(), (&self.iv).into())
                .decrypt_padded_mut::<cipher::block_padding::Pkcs7>(&mut input)
                .unwrap();
            slice.len()
        };

        input.truncate(slice_len);
        input
    }
}

#[derive(Clone)]
pub struct HlsSampleAesDecrypter {
    key: [u8; 16],
    iv: [u8; 16],
}

impl HlsSampleAesDecrypter {
    pub fn new(key: &[u8; 16], iv: &[u8; 16]) -> Self {
        Self { key: *key, iv: *iv }
    }

    pub fn increment_iv(&mut self) {
        self.iv = (u128::from_be_bytes(self.iv) + 1).to_be_bytes();
    }

    pub fn decrypt(&self, input: Vec<u8>) -> Vec<u8> {
        let mut input = std::io::Cursor::new(input);
        let mut output = Vec::new();
        iori_ssa::decrypt(&mut input, &mut output, self.key, self.iv).unwrap();
        output
    }
}
