use m3u8_rs::Key as HlsKey;
use openssl::symm::{Cipher, decrypt};

pub enum HlsEncryptionMethod {
    Aes128,
    SampleAes,
    None,
}
pub struct HlsDecrypt {
    key: Vec<u8>,
    iv: Option<Vec<u8>>,
    method: HlsEncryptionMethod,
}

impl HlsDecrypt {
    pub fn from_key(m3u8_key: HlsKey, key_content: Vec<u8>) -> Self {
        let iv = if let Some(encryption_iv) = m3u8_key.iv {
            Some(encryption_iv.as_bytes().to_vec())
        } else {
            None
        };

        match m3u8_key.method.as_str() {
            "NONE" => Self {
                key: vec![],
                iv: iv,
                method: HlsEncryptionMethod::None,
            },
            "AES-128" => Self {
                key: key_content,
                iv: iv,
                method: HlsEncryptionMethod::Aes128,
            },
            "SAMPLE-AES" => Self {
                key: key_content,
                iv: iv,
                method: HlsEncryptionMethod::SampleAes,
            },
            _ => panic!("Unsupported key method: {}", m3u8_key.method),
        }
    }

    pub fn decrypt(&self, buf: &[u8]) -> Vec<u8> {
        match self.method {
            HlsEncryptionMethod::None => buf.to_vec(),
            HlsEncryptionMethod::Aes128 => {
                if let Some(encryption_iv) = self.iv.clone() {
                    decrypt(Cipher::aes_128_cbc(), &self.key, Some(&encryption_iv), buf).unwrap()
                } else {
                    decrypt(Cipher::aes_128_cbc(), &self.key, None, buf).unwrap()
                }
            }
            HlsEncryptionMethod::SampleAes => {
                decrypt(Cipher::aes_128_cbc(), &self.key[..], None, buf).unwrap()
            }
        }
    }
}
