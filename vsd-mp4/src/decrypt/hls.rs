use cipher::{BlockDecryptMut, KeyIvInit};

type Aes128Cbc = cbc::Decryptor<aes::Aes128>;

pub struct HlsAes128Decrypter {
    key: [u8; 16],
    iv: [u8; 16],
}

impl HlsAes128Decrypter {
    pub fn new(key: &[u8; 16], iv: &[u8; 16]) -> Self {
        Self { key: *key, iv: *iv }
    }

    pub fn decrypt(&self, mut input: Vec<u8>) -> Vec<u8> {
        Aes128Cbc::new((&self.key).into(), (&self.iv).into())
            .decrypt_padded_mut::<cipher::block_padding::Pkcs7>(&mut input)
            .unwrap();
        input
    }
}
