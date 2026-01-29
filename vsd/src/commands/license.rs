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

#[derive(Args, Clone, Debug)]
/// Request content keys from a license server.
pub struct License {
    /// PSSH data input.
    /// Can be an init file path, playlist url or base64 encoded PSSH box.
    #[arg(required = true, value_name = "PATH|URL|BASE64")]
    input: String,

    /// Extra headers for license request in same format as curl.
    ///
    /// This option can be used multiple times.
    #[arg(short = 'H', long = "header", value_name = "KEY:VALUE", value_parser = Self::parse_header)]
    headers: Vec<(HeaderName, HeaderValue)>,

    /// Path to the Playready device (.prd) file.
    #[arg(long, value_name = "PRD")]
    playready_device: Option<PathBuf>,

    /// Path to the Widevine device (.wvd) file.
    #[arg(long, value_name = "WVD")]
    widevine_device: Option<PathBuf>,

    /// Playready license server URL.
    #[arg(long, value_name = "URL")]
    playready_url: Option<Url>,

    /// Widevine license server URL.
    #[arg(long, value_name = "URL")]
    widevine_url: Option<Url>,
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
        let mut pssh_data = HashSet::new();

        if Path::new(&self.input).exists() {
            let pssh = PsshBox::from_init(&fs::read(&self.input)?)?;
            for data in pssh.data {
                pssh_data.insert(data.data);
            }
        } else if let Ok(_url) = self.input.parse::<Url>() {
            unimplemented!()
        } else if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(&self.input) {
            pssh_data.insert(data);
        } else {
            bail!("Unable to determine the INPUT type.");
        }

        let client = Client::builder()
            .default_headers(HeaderMap::from_iter(self.headers))
            .build()?;

        for pssh in pssh_data {
            match Self::system_id(&pssh)? {
                SystemId::PlayReady => {
                    let Some(device_path) = &self.playready_device else {
                        bail!("Playready device (.prd) path not provided.");
                    };
                    let Some(license_url) = &self.playready_url else {
                        bail!("Playready license url not provided.");
                    };
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
                        println!("[{}] {}:{}", "CONTENT".green(), kid, key);
                    }
                }
                SystemId::WideVine => {
                    let Some(device_path) = &self.widevine_device else {
                        bail!("Widevine device (.wvd) path not provided.");
                    };
                    let Some(license_url) = &self.widevine_url else {
                        bail!("Widevine license url not provided.");
                    };
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
                        info!(
                            "[{}] {}:{}",
                            format!("{:?}", key.typ).green(),
                            hex::encode(key.kid),
                            hex::encode(key.key)
                        );
                    }
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}
