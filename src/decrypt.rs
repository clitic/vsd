use anyhow::Result;
use m3u8_rs::KeyMethod;
use openssl::symm::{decrypt, Cipher};

pub struct HlsDecrypt {
    key: Vec<u8>,
    iv: Option<Vec<u8>>,
    method: KeyMethod,
}

impl HlsDecrypt {
    pub fn from_key(key: m3u8_rs::Key, key_content: Vec<u8>) -> Self {
        let iv = key
            .iv
            .map(|encryption_iv| encryption_iv.as_bytes().to_vec());

        match key.method {
            KeyMethod::AES128 | KeyMethod::SampleAES => Self {
                key: key_content,
                iv,
                method: key.method,
            },
            KeyMethod::None => Self {
                key: vec![],
                iv,
                method: key.method,
            },
            KeyMethod::Other(x) => {
                panic!("Unsupported key method {}", x);
            }
        }
    }

    pub fn decrypt(&self, buf: &[u8]) -> Result<Vec<u8>> {
        match self.method {
            KeyMethod::AES128 => {
                if let Some(encryption_iv) = self.iv.clone() {
                    Ok(decrypt(
                        Cipher::aes_128_cbc(),
                        &self.key,
                        Some(&encryption_iv),
                        buf,
                    )?)
                } else {
                    Ok(decrypt(Cipher::aes_128_cbc(), &self.key, None, buf)?)
                }
            }
            KeyMethod::SampleAES => {
                let mut new_buf = vec![];

                for byte in buf {
                    let mut data = if let Some(encryption_iv) = self.iv.clone() {
                        decrypt(
                            Cipher::aes_128_cbc(),
                            &self.key,
                            Some(&encryption_iv),
                            &[byte.to_owned()],
                        )
                    } else {
                        decrypt(Cipher::aes_128_cbc(), &self.key, None, &[byte.to_owned()])
                    };

                    if let Ok(bytes) = &mut data {
                        new_buf.append(bytes);
                    } else {
                        new_buf.push(byte.to_owned());
                    }
                }

                Ok(new_buf)
            }

            _ => Ok(buf.to_vec()),
        }
    }
}
