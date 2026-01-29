use crate::utils;
use anyhow::{Result, bail};
use clap::Args;
use reqwest::{Client, Url};
use std::{fs::File, path::Path};
use widevine::{Cdm, Device, Key, LicenseType, Pssh};

#[derive(Args, Clone, Debug)]
pub struct License {
    #[arg(required = true)]
    input: String,
}

impl License {
    fn parse_input(&self) -> Result<Vec<Vec<u8>>> {
        let init_segments = Vec::new();

        if Path::new(&self.input).exists() {
        } else if let Ok(url) = self.input.parse::<Url>() {
        } else if let Ok(data) = utils::decode_base64(&self.input) {
        } else {
            bail!("Unable to determine the INPUT type.");
        }

        Ok(init_segments)
    }

    pub async fn execute(self) -> Result<()> {
        // let device = Device::read_wvd(File::open(WIDEVINE_DEVICE)?)?;
        // let cdm = Cdm::new(device);

        // let pssh = Pssh::from_b64(PSSH_DATA_2)?;
        // let request = cdm
        //     .open()
        //     .get_license_request(pssh, LicenseType::STREAMING)?;
        // let challenge = request.challenge()?;

        // let resp_data = Client::new()
        //     .post(L_URL_2)
        //     .body(challenge)
        //     .send()
        //     .await?
        //     .bytes()
        //     .await?
        //     .to_vec();

        // let keys = request.get_keys(&resp_data)?;
        // let keys: Vec<Key> = unsafe { std::mem::transmute(keys) };

        // for key in keys {
        //     println!(
        //         "[{:?}] {}:{}",
        //         key.typ,
        //         hex::encode(key.kid),
        //         hex::encode(key.key)
        //     );
        // }

        Ok(())
    }
}
