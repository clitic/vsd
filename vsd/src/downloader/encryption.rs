use crate::playlist::{Key, KeyMethod, MediaPlaylist, Segment};
use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
use anyhow::{Result, anyhow, bail};
use kdam::term::Colorizer;
use reqwest::{Url, blocking::Client, header};
use std::collections::{HashMap, HashSet};
use vsd_mp4::pssh::Pssh;

pub fn check_unsupported_encryptions(streams: &Vec<MediaPlaylist>) -> Result<()> {
    for stream in streams {
        if let Some(Segment { key: Some(x), .. }) = stream.segments.first() {
            match &x.method {
                KeyMethod::Other(x) => bail!(
                    "{} decryption is not supported. Use {} flag to download encrypted streams.",
                    x,
                    "--no-decrypt".colorize("bold green")
                ),
                KeyMethod::SampleAes => {
                    if stream.is_hls() {
                        bail!(
                            "sample-aes decryption is not supported. Use {} flag to download encrypted streams.",
                            "--no-decrypt".colorize("bold green")
                        );
                    }
                }
                _ => (),
            }

            if stream.is_hls() {
                if let Some(key_format) = &x.key_format {
                    bail!(
                        "{} key format is not supported. Use {} flag to download encrypted streams.",
                        key_format,
                        "--no-decrypt".colorize("bold green")
                    );
                }
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

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

fn decrypt_aes_128_cbc(input: &mut [u8], key: &[u8], iv: Option<&Vec<u8>>) -> Result<Vec<u8>> {
    let key_length = key.len();

    if key_length != 16 {
        bail!("invalid key size i.e. {} but expected size 16.", key_length);
    }

    let mut key_c = [0_u8; 16];
    key_c.copy_from_slice(key);

    let mut iv_c = [0_u8; 16];

    if let Some(iv) = iv {
        let iv_length = key.len();

        if iv_length != 16 {
            bail!("invalid iv size i.e. {} but expected size 16.", iv_length);
        }

        iv_c.copy_from_slice(iv);
    }

    Aes128CbcDec::new(&key_c.into(), &iv_c.into())
        .decrypt_padded_mut::<Pkcs7>(input)
        .map(|x| x.to_vec())
        .map_err(|x| anyhow!("{}", x))
}

#[derive(Clone)]
pub enum Decrypter {
    Aes128(Vec<u8>, Option<String>),
    Cenc(HashMap<String, String>),
    None,
}

impl Decrypter {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn decrypt(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
        Ok(match self {
            Decrypter::Aes128(key, iv) => {
                let iv = if let Some(x) = iv {
                    Some(hex::decode(x.trim_start_matches("0x"))?)
                } else {
                    None
                };

                decrypt_aes_128_cbc(&mut data, key, iv.as_ref())?
            }
            Decrypter::Cenc(kid_key_pairs) => {
                mp4decrypt::mp4decrypt(&data, kid_key_pairs, None)
                    .map_err(|x| anyhow!(x))?
            }
            Decrypter::None => data,
        })
    }
}
