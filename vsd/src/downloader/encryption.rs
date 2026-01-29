use crate::playlist::{KeyMethod, MediaPlaylist, Segment};
use anyhow::{Result, bail};
use colored::Colorize;
use log::info;
use reqwest::{Client, Url};
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

    let mut pssh_data_hash = HashSet::new();

    for stream in streams {
        let base_url = base_url
            .clone()
            .unwrap_or(stream.uri.parse::<Url>().unwrap());
        let Some(init_seg) = stream.fetch_init_seg(&base_url, client, query).await? else {
            continue;
        };
        let pssh = PsshBox::from_init(&init_seg)?;

        for data in pssh.data {
            let hash = blake3::hash(&data.data).to_hex()[..7].to_owned();
            if pssh_data_hash.contains(&hash) {
                continue;
            }

            pssh_data_hash.insert(hash);
            info!(
                "DrmPsh [{}] {}",
                data.system_id.to_string().magenta(),
                data.as_base64(),
            );
            for kid in &data.key_ids {
                info!(
                    "DrmKid [{}] {}{}",
                    data.system_id.to_string().magenta(),
                    kid.uuid(),
                    if default_kids.contains(&kid.0) {
                        " (required)".bold().red()
                    } else {
                        "".normal()
                    },
                );
            }
        }
    }

    Ok(default_kids)
}
