use crate::playlist::{Key, KeyMethod, MediaPlaylist, Segment};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use anyhow::{anyhow, bail, Result};
use kdam::term::Colorizer;
use reqwest::{blocking::Client, header, Url};
use std::collections::{HashMap, HashSet};
use vsd_mp4::pssh::Pssh;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

pub fn check_unsupported_encryptions(streams: &Vec<MediaPlaylist>) -> Result<()> {
    for stream in streams {
        if let Some(Segment { key: Some(x), .. }) = stream.segments.first() {
            if let KeyMethod::Other(x) = &x.method {
                bail!(
                    "{} decryption is not supported. Use {} flag to download encrypted streams.",
                    x,
                    "--no-decrypt".colorize("bold green")
                );
            }
        }
    }

    Ok(())
}

pub fn check_key_exists_for_kid(
    decrypter: &Decrypter,
    default_kids: &HashSet<String>,
) -> Result<()> {
    let user_kids = match decrypter {
        Decrypter::Mp4Decrypt(kid_key_pairs) => kid_key_pairs
            .keys()
            .map(|x| x.to_owned())
            .collect::<Vec<String>>(),
        _ => Vec::new(),
    };

    for kid in default_kids {
        if !user_kids.iter().any(|x| x == kid) {
            bail!(
                "use {} flag to specify content decryption keys for at least * pre-fixed key ids.",
                "--key".colorize("bold green")
            );
        }
    }

    Ok(())
}

pub fn extract_default_kids(
    base_url: &Option<Url>,
    client: &Client,
    streams: &Vec<MediaPlaylist>,
    query: &HashMap<String, String>,
) -> Result<HashSet<String>> {
    let mut default_kids = HashSet::new();

    for stream in streams {
        if let Some(Segment {
            key: Some(Key {
                default_kid: Some(x),
                ..
            }),
            ..
        }) = stream.segments.first()
        {
            default_kids.insert(x.replace('-', ""));
        }
    }

    let mut parsed_kids = HashSet::new();

    for stream in streams {
        let stream_base_url = base_url
            .clone()
            .unwrap_or(stream.uri.parse::<Url>().unwrap());

        if let Some(Segment { map: Some(x), .. }) = stream.segments.first() {
            let url = stream_base_url.join(&x.uri)?;
            let mut request = client.get(url).query(query);

            if let Some(range) = &x.range {
                request = request.header(header::RANGE, range.as_header_value());
            }

            let response = request.send()?;
            let pssh = Pssh::new(&response.bytes()?).map_err(|x| anyhow!(x))?;

            for kid in pssh.key_ids {
                if !parsed_kids.contains(&kid.value) {
                    parsed_kids.insert(kid.value.clone());
                    println!(
                        "      {} [{:>9}] {} {}",
                        "KeyId".colorize("bold red"),
                        kid.system_type.to_string(),
                        kid.uuid(),
                        if default_kids.contains(&kid.value) {
                            "(required)"
                        } else {
                            ""
                        },
                    );
                }
            }
        }
    }

    Ok(default_kids)
}

#[derive(Clone, Debug)]
pub enum EncryptionType {
    Aes128,
    NotDefined,
    SampleAes,
}

#[derive(Clone, Debug)]
pub enum Decrypter {
    HlsAes([u8; 16], [u8; 16], EncryptionType),
    Mp4Decrypt(HashMap<String, String>),
    None,
}

impl std::fmt::Display for Decrypter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HlsAes(_, _, _) => write!(f, "hls-aes"),
            Self::Mp4Decrypt(_) => write!(f, "mp4decrypt"),
            Self::None => write!(f, "none"),
        }
    }
}

impl Decrypter {
    pub fn new_hls_aes(key: [u8; 16], iv: [u8; 16], enc_type: &KeyMethod) -> Self {
        let enc_type = match enc_type {
            KeyMethod::Aes128 => EncryptionType::Aes128,
            KeyMethod::SampleAes => EncryptionType::SampleAes,
            _ => EncryptionType::NotDefined,
        };

        Self::HlsAes(key, iv, enc_type)
    }

    pub fn is_hls_aes_and_not_defined(&self) -> bool {
        if let Self::HlsAes(_, _, enc_type) = self {
            matches!(enc_type, EncryptionType::NotDefined)
        } else {
            false
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn update_enc_type(&mut self, method: &KeyMethod) {
        if let Self::HlsAes(_, _, enc_type) = self {
            *enc_type = match method {
                KeyMethod::Aes128 => EncryptionType::Aes128,
                KeyMethod::SampleAes => EncryptionType::SampleAes,
                _ => panic!("trying to create a non aes hls decrypter."),
            };
        }
    }

    pub fn increment_iv(&mut self) {
        if let Self::HlsAes(_, iv, _) = self {
            *iv = (u128::from_be_bytes(*iv) + 1).to_be_bytes();
        }
    }

    pub fn update_iv(&mut self, sequence: u64) {
        if let Self::HlsAes(_, iv, _) = self {
            *iv = (sequence as u128).to_be_bytes();
        }
    }

    pub fn decrypt(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
        Ok(match self {
            Decrypter::HlsAes(key, iv, enc_type) => match enc_type {
                EncryptionType::Aes128 => Aes128CbcDec::new(key.into(), iv.into())
                    .decrypt_padded_mut::<Pkcs7>(&mut data)
                    .map(|x| x.to_vec())
                    .map_err(|x| anyhow!("{}", x))?,
                EncryptionType::NotDefined => data,
                EncryptionType::SampleAes => {
                    let mut reader = std::io::Cursor::new(data);
                    let mut writer = Vec::new();
                    iori_ssa::decrypt(&mut reader, &mut writer, *key, *iv)?;
                    writer
                }
            },
            Decrypter::Mp4Decrypt(kid_key_pairs) => {
                mp4decrypt::mp4decrypt(&data, kid_key_pairs, None).map_err(|x| anyhow!(x))?
            }
            Decrypter::None => data,
        })
    }
}
