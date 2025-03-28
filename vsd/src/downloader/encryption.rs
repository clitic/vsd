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
    keys: &[(Option<String>, String)],
    default_kids: &HashSet<String>,
) -> Result<()> {
    let user_kids = keys.iter().flat_map(|x| x.0.as_ref());

    for kid in default_kids {
        if !user_kids.clone().any(|x| x == kid) {
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
            let mut request = client.get(url);

            if let Some(range) = &x.range {
                request = request.header(header::RANGE, range.as_header_value());
            }

            let response = request.send()?;
            let pssh = Pssh::new(&response.bytes()?).map_err(|x| anyhow!(x))?;

            for kid in pssh.key_ids {
                if !parsed_kids.contains(&kid.value) {
                    parsed_kids.insert(kid.value.clone());
                    println!(
                        "      {} {} {} ({})",
                        "KeyId".colorize("bold green"),
                        if default_kids.contains(&kid.value) {
                            "*"
                        } else {
                            " "
                        },
                        kid.uuid(),
                        kid.system_type,
                    );
                }
            }
        }
    }

    Ok(default_kids)
}

#[derive(Clone)]
pub enum EncryptionType {
    Aes128,
    SampleAes,
}

#[derive(Clone)]
pub enum Decrypter {
    HlsAes([u8; 16], [u8; 16], EncryptionType),
    Mp4Decrypt(HashMap<String, String>),
    None,
}

impl Decrypter {
    pub fn new_hls_aes(key: [u8; 16], iv: [u8; 16], method: &KeyMethod) -> Result<Self> {
        let enc_type = match method {
            KeyMethod::Aes128 => EncryptionType::Aes128,
            KeyMethod::SampleAes => EncryptionType::SampleAes,
            _ => panic!("trying to create a non aes decrypter."),
        };

        Ok(Self::HlsAes(key, iv, enc_type))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn increment_iv(&mut self) {
        if let Self::HlsAes(_, iv, _) = self {
            *iv = (u128::from_be_bytes(*iv) + 1).to_be_bytes();
        }
    }

    pub fn decrypt(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
        Ok(match self {
            Decrypter::HlsAes(key, iv, enc_type) => match enc_type {
                EncryptionType::Aes128 => Aes128CbcDec::new(key.into(), iv.into())
                    .decrypt_padded_mut::<Pkcs7>(&mut data)
                    .map(|x| x.to_vec())
                    .map_err(|x| anyhow!("{}", x))?,
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
