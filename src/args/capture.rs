use clap::Args;

/// Capture requests made to fetch playlists.
#[derive(Debug, Clone, Args)]
#[clap(long_about = "Capture requests made to fetch playlists.\n\n\
*Requires* any one of these to be installed:\n\
1. chrome - https://www.google.com/chrome\n\
2. chromium - https://www.chromium.org/getting-involved/download-chromium\n\n\
Launch Google Chrome to capture requests made to fetch .m3u8 (HLS) and .mpd (Dash) playlists. \
This is done by reading the request response sent by chrome to server. \
This command might not work always as expected.")]
pub struct Capture {
    /// https:// | http://
    #[clap(required = true)]
    pub url: String,

    /// Launch Google Chrome without a window for interaction.
    #[clap(long)]
    pub headless: bool,
}
