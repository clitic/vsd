use kdam::term::Colorizer;
use m3u8_rs::Key as HlsKey;
use openssl::symm::{decrypt, Cipher};

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
            "SAMPLE-AES" => {
                println!(
                    "{}: SAMPLE-AES encrypted playlists are not supported.",
                    "Error".colorize("bold red")
                );
                std::process::exit(1);

                // Self {
                //     key: key_content,
                //     iv: iv,
                //     method: HlsEncryptionMethod::SampleAes,
                // }
            }
            _ => {
                println!(
                    "{}: Unsupported key method {}",
                    "Error".colorize("bold red"),
                    m3u8_key.method.colorize("bold yellow")
                );
                std::process::exit(1);
            }
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
                let mut new_buf = vec![];

                for byte in buf {
                    let data = if let Some(encryption_iv) = self.iv.clone() {
                        decrypt(
                            Cipher::aes_128_cbc(),
                            &self.key,
                            Some(&encryption_iv),
                            &[byte.to_owned()],
                        )
                    } else {
                        decrypt(Cipher::aes_128_cbc(), &self.key, None, &[byte.to_owned()])
                    };

                    if data.is_ok() {
                        new_buf.append(&mut data.unwrap());
                    } else {
                        new_buf.push(byte.to_owned());
                    }
                }

                new_buf
            }
        }
    }
}
