use anyhow::{Result, bail};
use base64::Engine;
use clap::Args;
use colored::Colorize;
use log::info;
use reqwest::{
    Client, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::{
    collections::HashSet,
    fs::{self, File},
    path::{Path, PathBuf},
};
use vsd_mp4::pssh::{PsshBox, SystemId};

use crate::Downloader;

/// Request content keys from a license server.
#[derive(Args, Clone, Debug)]
pub struct License {
    /// HTTP(S):// | INIT.mp4 | PSSH_BASE64
    #[arg(required = true)]
    input: String,

    /// Additional headers for license request in same format as curl.
    ///
    /// This option can be used multiple times.
    #[arg(short = 'H', long = "header", value_name = "KEY:VALUE", value_parser = Self::parse_header)]
    headers: Vec<(HeaderName, HeaderValue)>,

    /// Path to the playready device (.prd) file.
    #[arg(long, value_name = "PRD", help_heading = "Playready Options")]
    playready_device: Option<PathBuf>,

    /// Playready license server URL.
    #[arg(long, value_name = "URL", help_heading = "Playready Options")]
    playready_url: Option<Url>,

    /// Skip playready license request.
    #[arg(long, help_heading = "Playready Options")]
    skip_playready: bool,

    /// Path to the widevine device (.wvd) file.
    #[arg(long, value_name = "WVD", help_heading = "Widevine Options")]
    widevine_device: Option<PathBuf>,

    /// Widevine license server URL.
    #[arg(long, value_name = "URL", help_heading = "Widevine Options")]
    widevine_url: Option<Url>,

    /// Skip widevine license request.
    #[arg(long, help_heading = "Widevine Options")]
    skip_widevine: bool,
}

impl License {
    fn parse_header(value: &str) -> Result<(HeaderName, HeaderValue)> {
        if let Some((k, v)) = value.split_once(':') {
            Ok((k.trim().parse()?, v.trim().parse()?))
        } else {
            bail!("Expected 'KEY:VALUE' but found '{}'.", value);
        }
    }

    fn system_id(bytes: &[u8]) -> Result<SystemId> {
        if bytes.len() < 28 {
            bail!("Data too short to be a valid PSSH box.");
        }

        let box_type = &bytes[4..8];
        if box_type != b"pssh" {
            bail!(
                "Expected 'pssh' box type but found '{}'.",
                String::from_utf8_lossy(box_type)
            );
        }

        let system_id = hex::encode(&bytes[12..28]);
        match system_id.as_str() {
            "9a04f07998404286ab92e65be0885f95" => Ok(SystemId::PlayReady),
            "edef8ba979d64acea3c827dcd51d21ed" => Ok(SystemId::WideVine),
            _ => bail!("'{}' system id not supported.", system_id),
        }
    }

    pub async fn execute(self) -> Result<()> {
        let client = Client::builder()
            .default_headers(HeaderMap::from_iter(self.headers))
            .build()?;
        let mut pssh_data = HashSet::new();

        if Path::new(&self.input).exists() {
            PsshBox::from_init(&fs::read(&self.input)?)?
                .data
                .into_iter()
                .for_each(|x| {
                    let _ = pssh_data.insert(x.data);
                });
        } else if let Ok(url) = self.input.parse::<Url>() {
            pssh_data = Downloader::new(url.as_str(), &client)
                .pssh_playlist()
                .await?;
        } else if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(&self.input) {
            pssh_data.insert(data);
        } else {
            bail!("Unable to determine the INPUT type.");
        }

        for pssh in pssh_data {
            match Self::system_id(&pssh)? {
                SystemId::PlayReady => {
                    if self.skip_playready {
                        continue;
                    }
                    let Some(device_path) = &self.playready_device else {
                        bail!("Playready device (.prd) path not provided.");
                    };
                    let Some(license_url) = &self.playready_url else {
                        bail!("Playready license url not provided.");
                    };
                    println!(
                        "DrmPsh [{}] {}",
                        "prd".magenta(),
                        base64::engine::general_purpose::STANDARD.encode(&pssh)
                    );
                    let pssh = playready::Pssh::from_bytes(&pssh)?;
                    let device = playready::Device::from_prd(device_path)?;
                    let cdm = playready::Cdm::from_device(device);
                    let session = cdm.open_session();
                    let challenge = session.get_license_challenge(pssh.wrm_headers()[0].clone())?;
                    let response = client
                        .post(license_url.to_owned())
                        .header(reqwest::header::CONTENT_TYPE, "text/xml; charset=utf-8")
                        .body(challenge)
                        .send()
                        .await?;
                    let status = response.status();

                    if status.is_client_error() || status.is_server_error() {
                        bail!(
                            "Playready license request failed ({}): '{}'",
                            status,
                            response.text().await?
                        );
                    }

                    let data = response.text().await?;
                    let keys = session.get_keys_from_challenge_response(&data)?;

                    for (kid, key) in &keys {
                        println!("DrmKey [{}] {}:{}", "prd".magenta(), kid, key);
                    }
                }
                SystemId::WideVine => {
                    if self.skip_widevine {
                        continue;
                    }
                    let Some(device_path) = &self.widevine_device else {
                        bail!("Widevine device (.wvd) path not provided.");
                    };
                    let Some(license_url) = &self.widevine_url else {
                        bail!("Widevine license url not provided.");
                    };
                    println!(
                        "DrmPsh [{}] {}",
                        "wvd".magenta(),
                        base64::engine::general_purpose::STANDARD.encode(&pssh)
                    );
                    let pssh = widevine::Pssh::from_bytes(&pssh)?;
                    let device = widevine::Device::read_wvd(File::open(device_path)?)?;
                    let cdm = widevine::Cdm::new(device);
                    let session = cdm
                        .open()
                        .get_license_request(pssh, widevine::LicenseType::STREAMING)?;
                    let challenge = session.challenge()?;
                    let response = client
                        .post(license_url.to_owned())
                        .body(challenge)
                        .send()
                        .await?;
                    let status = response.status();

                    if status.is_client_error() || status.is_server_error() {
                        bail!(
                            "Widevine license request failed ({}): '{}'",
                            status,
                            response.text().await?
                        );
                    }

                    let data = response.bytes().await?;
                    let keys = session.get_keys(&data)?;
                    let keys: Vec<widevine::Key> = unsafe { std::mem::transmute(keys) };

                    for key in keys {
                        if let widevine::KeyType::CONTENT = key.typ {
                            info!(
                                "DrmKey [{}] {}:{}",
                                "wvd".magenta(),
                                hex::encode(key.kid),
                                hex::encode(key.key)
                            );
                        }
                    }
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}
