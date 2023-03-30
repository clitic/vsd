#![cfg(feature = "chrome")]

use super::utils;
use anyhow::Result;
use clap::Args;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use kdam::term::Colorizer;

/// Capture requests made to fetch playlists.
#[derive(Debug, Clone, Args)]
#[clap(long_about = "Capture requests made to fetch playlists.\n\n\
Requires any one of these to be installed:\n\
1. chrome - https://www.google.com/chrome\n\
2. chromium - https://www.chromium.org/getting-involved/download-chromium\n\n\
Launch Google Chrome to capture requests made to fetch .m3u8 (HLS) and .mpd (Dash) playlists. \
This is done by reading the request response sent by chrome to server. \
This command might not work always as expected.")]
pub struct Capture {
    /// https:// | http://
    #[arg(required = true)]
    url: String,

    /// Launch Google Chrome without a window for interaction.
    #[arg(long)]
    headless: bool,
}

impl Capture {
    pub fn perform(&self) -> Result<()> {
        utils::chrome_launch_message(self.headless);

        let browser = Browser::new(
            LaunchOptionsBuilder::default()
                .headless(self.headless)
                .build()?,
        )?;

        let tab = browser.new_tab()?;

        tab.register_response_handling(
            "vsd-capture",
            Box::new(move |params, _| {
                let url = params.response.url.split('?').next().unwrap();

                if url.contains(".m3u") || url.contains(".mpd") {
                    println!(
                        "{}\n{}",
                        "-".repeat(kdam::term::get_columns_or(10) as usize)
                            .colorize("#FFA500"),
                        url
                    );
                }
            }),
        )?;

        tab.navigate_to(&self.url)?;
        utils::chrome_warning_message();
        std::thread::sleep(std::time::Duration::from_secs(60 * 3));
        Ok(())
    }
}
