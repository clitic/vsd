use anyhow::{anyhow, bail, Result};
use m3u8_rs::KeyMethod;
use openssl::symm::{decrypt, Cipher};
use std::collections::HashMap;

pub enum EncryptionMethod {
    None,
    AES128,
    // SampleAES,
    CENC,
}

impl EncryptionMethod {
    pub fn _is_cenc(&self) -> bool {
        if let Self::CENC = self {
            true
        } else {
            false
        }
    }
}
pub struct Decrypter {
    key: Vec<u8>,
    iv: Option<Vec<u8>>,
    method: EncryptionMethod,
}

impl Decrypter {
    pub fn from_key(key: &m3u8_rs::Key, key_content: &[u8]) -> Result<Self> {
        match &key.method {
            KeyMethod::AES128 => Ok(Self {
                key: key_content.to_vec(),
                iv: key
                    .iv
                    .clone()
                    .map(|encryption_iv| encryption_iv.as_bytes().to_vec()),
                method: EncryptionMethod::AES128,
            }),
            KeyMethod::None => Ok(Self {
                key: vec![],
                iv: None,
                method: EncryptionMethod::None,
            }),
            KeyMethod::SampleAES => bail!("SAMPLE-AES decryption is not supported."),
            KeyMethod::Other(x) => {
                if x == "CENC" {
                    Ok(Self {
                        key: vec![],
                        iv: None,
                        method: EncryptionMethod::CENC,
                    })
                } else {
                    bail!("{} decryption is not supported.", x)
                }
            }
        }
    }

    pub fn decrypt(&self, data: &[u8], keys: Option<HashMap<String, String>>) -> Result<Vec<u8>> {
        match self.method {
            EncryptionMethod::AES128 => {
                if let Some(encryption_iv) = self.iv.clone() {
                    Ok(decrypt(
                        Cipher::aes_128_cbc(),
                        &self.key,
                        Some(&encryption_iv),
                        data,
                    )?)
                } else {
                    Ok(decrypt(Cipher::aes_128_cbc(), &self.key, None, data)?)
                }
            }
            EncryptionMethod::CENC => {
                if let Some(keys) = keys {
                    Ok(mp4decrypt::mp4decrypt(data, keys, None).map_err(|x| anyhow!(x))?)
                } else {
                    bail!("CENC encryption can't be decrypted without keys.")
                }
            }
            _ => Ok(data.to_vec()),
        }
    }
}
