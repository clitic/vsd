use super::utils;
use anyhow::{anyhow, Result};
use clap::Args;
use headless_chrome::protocol::network::events::ResourceType;
use headless_chrome::protocol::network::methods::GetResponseBodyReturnObject;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use kdam::term::Colorizer;
use std::fs::File;
use std::io::Write;

/// Collect playlists and subtitles from a website and save them locally.
#[derive(Debug, Clone, Args)]
#[clap(
    long_about = "Collect playlists and subtitles from a website and save them locally.\n\n\
Requires any one of these to be installed:\n\
1. chrome - https://www.google.com/chrome\n\
2. chromium - https://www.chromium.org/getting-involved/download-chromium\n\n\
Launch Google Chrome and collect .m3u8 (HLS), .mpd (Dash) and subtitles from a website and save them locally. \
This is done by reading the request response sent by chrome to server. \
This command might not work always as expected."
)]
pub struct Collect {
    /// https:// | http://
    #[arg(required = true)]
    url: String,

    /// Launch Google Chrome without a window for interaction.
    #[arg(long)]
    headless: bool,

    /// Build http links for all uri(s) present in HLS playlists before saving it.
    #[arg(long)]
    build: bool,
}

impl Collect {
    pub fn perform(&self) -> Result<()> {
        utils::chrome_launch_message(self.headless);

        let browser = Browser::new(
            LaunchOptionsBuilder::default()
                .headless(self.headless)
                .build()
                .map_err(|e| anyhow!(e))?,
        )
        .map_err(|e| anyhow!(e))?;

        let tab = browser.wait_for_initial_tab().map_err(|e| anyhow!(e))?;
        let build = self.build;

        tab.enable_response_handling(Box::new(move |params, get_response_body| {
            if params._type == ResourceType::XHR || params._type == ResourceType::Fetch {
                let url = params.response.url.split('?').next().unwrap();

                if url.contains(".m3u")
                    || url.contains(".mpd")
                    || url.contains(".vtt")
                    || url.contains(".srt")
                {
                    if let Ok(body) = get_response_body() {
                        save_to_disk(url, body, build).unwrap();
                    }
                }
            }
        }))
        .map_err(|e| anyhow!(e))?;

        tab.navigate_to(&self.url).map_err(|e| anyhow!(e))?;

        println!(
            "Using {} method for collection which might {} as expected.",
            "COMMON".colorize("cyan"),
            "not work".colorize("bold red")
        );

        utils::chrome_warning_message();
        std::thread::sleep(std::time::Duration::from_secs(60 * 3));
        Ok(())
    }
}

fn decode_body(body: GetResponseBodyReturnObject) -> Result<Vec<u8>> {
    if body.base64_encoded {
        Ok(openssl::base64::decode_block(&body.body)?)
    } else {
        Ok(body.body.as_bytes().to_vec())
    }
}

fn save_to_disk(url: &str, body: GetResponseBodyReturnObject, build: bool) -> Result<()> {
    if url.contains(".m3u") {
        let file = utils::filepath(url, "m3u8");

        if build {
            utils::build_links(&decode_body(body)?, &file, url)?;
            println!(
                "Saved {} playlist from {} to {}",
                "BUILDED HLS".colorize("cyan"),
                url,
                file.colorize("bold green")
            );
        } else {
            File::create(&file)?.write_all(&decode_body(body)?)?;
            println!(
                "Saved {} playlist from {} to {}",
                "HLS".colorize("cyan"),
                url,
                file.colorize("bold green")
            );
        }
    } else if url.contains(".mpd") {
        let file = utils::filepath(url, "mpd");
        File::create(&file)?.write_all(&decode_body(body)?)?;
        println!(
            "Saved {} playlist from {} to {}",
            "DASH".colorize("cyan"),
            url,
            file.colorize("bold green")
        );
    } else if url.contains(".vtt") {
        let file = utils::filepath(url, "vtt");
        File::create(&file)?.write_all(&decode_body(body)?)?;
        println!(
            "Saved {} from {} to {}",
            "SUBTITLES".colorize("cyan"),
            url,
            file.colorize("bold green")
        );
    } else if url.contains(".srt") {
        let file = utils::filepath(url, "srt");
        File::create(&file)?.write_all(&decode_body(body)?)?;
        println!(
            "Saved {} from {} to {}",
            "SUBTITLES".colorize("cyan"),
            url,
            file.colorize("bold green")
        );
    }

    Ok(())
}
