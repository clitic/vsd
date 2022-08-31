use anyhow::{bail, Result};
use m3u8_rs::KeyMethod;
use openssl::symm::{decrypt, Cipher};

pub struct HlsDecrypt {
    key: Vec<u8>,
    iv: Option<Vec<u8>>,
    method: KeyMethod,
}

impl HlsDecrypt {
    pub fn from_key(key: m3u8_rs::Key, key_content: Vec<u8>) -> Result<Self> {
        match key.method {
            KeyMethod::AES128 => Ok(Self {
                key: key_content,
                iv: key
                    .iv
                    .map(|encryption_iv| encryption_iv.as_bytes().to_vec()),
                method: key.method,
            }),
            KeyMethod::None => Ok(Self {
                key: vec![],
                iv: None,
                method: key.method,
            }),
            KeyMethod::SampleAES => bail!("SAMPLE-AES key method is not supported."),
            KeyMethod::Other(x) => bail!("Unsupported key method {}.", x),
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
            _ => Ok(buf.to_vec()),
        }
    }
}
