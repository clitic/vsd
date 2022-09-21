use clap::Args;

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
    #[clap(required = true)]
    pub url: String,

    /// Launch Google Chrome without a window for interaction.
    #[clap(long)]
    pub headless: bool,

    /// Build http links for all uri(s) present in HLS playlists before saving it.
    #[clap(long)]
    pub build: bool,
}
