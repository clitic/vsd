use crate::playlist::{KeyMethod, MediaPlaylist, Segment};
use anyhow::{Result, bail};
use colored::Colorize;
use log::info;
use reqwest::{Client, Url, header};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use vsd_mp4::{
    decrypt::{CencDecryptingProcessor, HlsAes128Decrypter, HlsSampleAesDecrypter},
    pssh::PsshBox,
};

#[derive(Clone)]
pub enum Decrypter {
    Aes128(HlsAes128Decrypter),
    Cenc(Arc<CencDecryptingProcessor>),
    SampleAes(HlsSampleAesDecrypter),
    None,
}

impl Decrypter {
    pub fn is_hls(&self) -> bool {
        matches!(self, Decrypter::Aes128(_) | Decrypter::SampleAes(_))
    }

    pub fn increment_iv(&mut self) {
        match self {
            Decrypter::Aes128(processor) => processor.increment_iv(),
            Decrypter::SampleAes(processor) => processor.increment_iv(),
            _ => (),
        }
    }

    pub fn decrypt(&self, input: Vec<u8>, init: Option<Vec<u8>>) -> Result<Vec<u8>> {
        Ok(match self {
            Decrypter::Cenc(processor) => processor.decrypt(input, init)?,
            Decrypter::Aes128(processor) => processor.decrypt(input),
            Decrypter::SampleAes(processor) => processor.decrypt(input),
            Decrypter::None => input,
        })
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
                request = request.header(header::RANGE, range);
            }

            let response = request.send().await?;
            let bytes = response.bytes().await?;

            // let default_kid = TencBox::from_init(&bytes)?.map(|x| x.default_kid_hex());
            let pssh = PsshBox::from_init(&bytes)?;

            for kid in pssh.key_ids {
                // if default_kid.as_deref() == Some("00000000000000000000000000000000")
                //     && matches!(kid.system_type, vsd_mp4::pssh::KeyIdSystemType::WideVine)
                // {
                //     default_kids.insert(kid.value.clone());
                // }

                if !parsed_kids.contains(&kid.value) {
                    parsed_kids.insert(kid.value.clone());
                    info!(
                        "DrmKid [{}] {}{}",
                        kid.system_type.to_string().magenta(),
                        kid.uuid(),
                        if default_kids.contains(&kid.value) {
                            " (required)".bold().red()
                        } else {
                            "".normal()
                        },
                    );
                }
            }
        }
    }

    Ok(default_kids)
}
