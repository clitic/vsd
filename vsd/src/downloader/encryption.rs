use crate::playlist::{KeyMethod, MediaPlaylist, Segment};
use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
use anyhow::{Result, anyhow, bail};
use colored::Colorize;
use log::info;
use mp4decrypt::Ap4CencDecryptingProcessor;
use reqwest::{Client, Url, header};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use vsd_mp4::{boxes::TencBox, pssh::Pssh};

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

#[derive(Clone)]
pub enum Decrypter {
    Aes128([u8; 16], [u8; 16]),
    CencCbcs(Arc<Ap4CencDecryptingProcessor>),
    SampleAes([u8; 16], [u8; 16]),
    None,
}

impl Decrypter {
    pub fn decrypt(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
        Ok(match self {
            Decrypter::CencCbcs(processor) => processor.decrypt(data, None)?,
            Decrypter::Aes128(key, iv) => Aes128CbcDec::new(key.into(), iv.into())
                .decrypt_padded_mut::<Pkcs7>(&mut data)
                .map(|x| x.to_vec())
                .map_err(|x| anyhow!("{}", x))?,
            Decrypter::SampleAes(key, iv) => {
                let mut reader = std::io::Cursor::new(data);
                let mut writer = Vec::new();
                iori_ssa::decrypt(&mut reader, &mut writer, *key, *iv)?;
                writer
            }
            Decrypter::None => data,
        })
    }

    pub fn increment_iv(&mut self) {
        if let Self::SampleAes(_, iv) = self {
            *iv = (u128::from_be_bytes(*iv) + 1).to_be_bytes();
        }
    }
}

pub fn check_key_exists_for_kid(
    keys: &HashMap<String, String>,
    default_kids: &HashSet<String>,
) -> Result<()> {
    let user_kids = keys.keys().map(|x| x.to_owned()).collect::<Vec<String>>();

    for kid in default_kids {
        if !user_kids.iter().any(|x| x == kid) {
            bail!(
                "use --keys flag to specify content decryption keys for at least required key ids ({}).",
                default_kids
                    .iter()
                    .map(|item| item.to_owned())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    Ok(())
}

pub fn check_unsupported_encryptions(streams: &Vec<MediaPlaylist>) -> Result<()> {
    for stream in streams {
        if let Some(Segment { key: Some(x), .. }) = stream.segments.first()
            && let KeyMethod::Other(x) = &x.method
        {
            bail!(
                "{} decryption is not supported. Use --no-decrypt flag to download encrypted streams.",
                x,
            );
        }
    }

    Ok(())
}

pub async fn extract_default_kids(
    base_url: &Option<Url>,
    client: &Client,
    streams: &Vec<MediaPlaylist>,
    query: &HashMap<String, String>,
) -> Result<HashSet<String>> {
    let mut default_kids = HashSet::new();

    for stream in streams {
        if let Some(default_kid) = stream.default_kid() {
            default_kids.insert(default_kid);
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

            let response = request.send().await?;
            let bytes = response.bytes().await?;

            let default_kid = TencBox::from_init(&bytes)?.map(|x| x.default_kid);
            let pssh = Pssh::new(&bytes).map_err(|x| anyhow!(x))?;

            for kid in pssh.key_ids {
                if default_kid == Some("00000000000000000000000000000000".to_owned())
                    && matches!(kid.system_type, vsd_mp4::pssh::KeyIdSystemType::WideVine)
                {
                    default_kids.insert(kid.value.clone());
                }

                if !parsed_kids.contains(&kid.value) {
                    parsed_kids.insert(kid.value.clone());
                    info!(
                        "Found {:>9} drm: {}{}",
                        kid.system_type.to_string(),
                        kid.uuid().bold(),
                        if default_kids.contains(&kid.value) {
                            " (required)"
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
