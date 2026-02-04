use crate::{Downloader, cookie::Cookies};
use anyhow::{Result, bail};
use clap::Args;
use reqwest::{
    Client, Proxy, Url,
    cookie::Jar,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};
use tokio::fs;

/// Download streams from DASH or HLS playlist.
#[derive(Args, Clone, Debug)]
pub struct Save {
    /// HTTP(S):// | .M3U8 | .MPD
    #[arg(required = true)]
    pub input: String,

    /// Base URL for resolving relative segment paths.
    ///
    /// Required for local playlist files. For remote playlists,
    /// the final redirected URL is used by default.
    #[arg(long)]
    pub base_url: Option<Url>,

    /// Working directory for temporary segment files.
    ///
    /// Defaults to the current directory.
    #[arg(short, long)]
    pub directory: Option<PathBuf>,

    /// Mux downloaded streams into a video container using ffmpeg (`.mp4`, `.mkv`, etc.).
    ///
    /// Overwrites existing files and deletes intermediate stream files after muxing.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Output parsed playlist metadata as JSON instead of downloading.
    #[arg(long)]
    pub parse: bool,

    /// Subtitle codec to use when muxing with ffmpeg.
    ///
    /// Defaults to `mov_text` for `.mp4` containers, `copy` for others.
    #[arg(
        long,
        value_name = "CODEC",
        default_value = "copy",
        hide_default_value = true
    )]
    pub subs_codec: String,

    /// Enable interactive stream selection with styled prompts.
    #[arg(
        short,
        long,
        help_heading = "Automation Options",
        conflicts_with = "interactive_raw"
    )]
    pub interactive: bool,

    /// Enable interactive stream selection with plain text prompts.
    #[arg(short = 'I', long, help_heading = "Automation Options")]
    pub interactive_raw: bool,

    /// Display all available streams without downloading.
    #[arg(short, long, help_heading = "Automation Options")]
    pub list_streams: bool,

    /// Stream selection filters for automatic mode.
    #[arg(
        short,
        long,
        value_name = "STREAMS",
        help_heading = "Automation Options",
        default_value = "v=best:s=en",
        long_help = "Stream selection filters for automatic mode.\n\n\
        SYNTAX:\n\n\
        `v={}:a={}:s={}` where `{}` (in priority order) can contain\n\n\
        |> all: select all streams.\n\
        |> skip: skip all streams or select inverter.\n\
        |> 1,2: indices obtained by --list-streams flag.\n\
        |> 1080p,1280x720: stream resolution.\n\
        |> en,fr: stream language.\n\n\
        EXAMPLES:\n\n\
        |> 1,2,3 (indices 1, 2, and 3)\n\
        |> v=skip:a=skip:s=all (all sub streams)\n\
        |> a:en:s=en (prefer en lang)\n\
        |> v=1080p:a=all:s=skip (1080p with all aud streams)\n"
    )]
    pub select_streams: String,

    /// Path to a netscape cookie file for authenticated requests.
    #[arg(long, value_name = "PATH", help_heading = "Client Options")]
    pub cookies: Option<PathBuf>,

    /// Additional headers for requests in same format as curl.
    ///
    /// This option can be used multiple times.
    #[arg(short = 'H', long = "header", value_name = "KEY:VALUE", help_heading = "Client Options", value_parser = Self::parse_header)]
    pub headers: Vec<(HeaderName, HeaderValue)>,

    /// Proxy server URL (HTTP, HTTPS, or SOCKS).
    #[arg(long, help_heading = "Client Options", value_parser = Self::parse_proxy)]
    pub proxy: Option<Proxy>,

    /// Additional query parameters for requests.
    #[arg(long, help_heading = "Client Options")]
    pub query: Option<String>,

    /// Decryption keys in `KID:KEY;…` hex format.
    #[arg(long, value_name = "KID:KEY;…", help_heading = "Decrypt Options", default_value = "", hide_default_value = true, value_parser = Self::parse_keys)]
    pub keys: HashMap<String, String>,

    /// Skip decryption and download encrypted streams as-is.
    ///
    /// Ignores `--output` when enabled.
    #[arg(long, help_heading = "Decrypt Options")]
    pub no_decrypt: bool,

    /// Skip segment merging and keep individual files.
    ///
    /// Ignores `--output` when enabled.
    #[arg(long, help_heading = "Download Options")]
    pub no_merge: bool,

    /// Maximum retry attempts per segment.
    #[arg(long, help_heading = "Download Options", default_value_t = 10)]
    pub retries: u8,

    /// Number of concurrent download threads (1–16).
    #[arg(short, long, help_heading = "Download Options", default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=16))]
    pub threads: u8,
}

impl Save {
    fn parse_header(s: &str) -> Result<(HeaderName, HeaderValue)> {
        if let Some((k, v)) = s.split_once(':') {
            Ok((k.trim().parse()?, v.trim().parse()?))
        } else {
            bail!("Expected 'KEY:VALUE' but found '{}'.", s);
        }
    }

    fn parse_proxy(s: &str) -> Result<Proxy> {
        Ok(Proxy::all(s)?)
    }

    fn parse_keys(s: &str) -> Result<HashMap<String, String>> {
        let mut keys = HashMap::new();

        if s.is_empty() {
            return Ok(keys);
        }

        for pair in s.split(';') {
            if let Some((kid, key)) = pair.split_once(':') {
                let kid = kid.to_ascii_lowercase().replace('-', "");
                let key = key.to_ascii_lowercase().replace('-', "");

                if kid.len() == 32
                    && key.len() == 32
                    && kid.chars().all(|c| c.is_ascii_hexdigit())
                    && key.chars().all(|c| c.is_ascii_hexdigit())
                {
                    keys.insert(kid, key);
                } else {
                    bail!("Expected 'KID:KEY;…' but found '{}'.", s);
                }
            }
        }

        Ok(keys)
    }

    pub async fn execute(self) -> Result<()> {
        let mut client = Client::builder()
            .default_headers(HeaderMap::from_iter(self.headers))
            .cookie_store(true)
            .timeout(Duration::from_secs(60));
        if let Some(path) = &self.cookies {
            let jar = Jar::default();
            let data = fs::read(path).await?;

            for cookie in Cookies::parse(&data)?.0 {
                jar.add_cookie_str(&cookie.to_header(), &cookie.url().parse::<Url>()?);
            }

            client = client.cookie_provider(Arc::new(jar));
        }
        if let Some(proxy) = self.proxy {
            client = client.proxy(proxy);
        }

        let mut dl = Downloader::new(self.input, client.build()?)
            .subs_codec(self.subs_codec)
            .select_streams(&self.select_streams)
            .keys(self.keys)
            .skip_decrypt(self.no_decrypt)
            .skip_merge(self.no_merge)
            .max_retries(self.retries)
            .max_threads(self.threads);

        if let Some(base_url) = self.base_url {
            dl = dl.base_url(base_url);
        }
        if let Some(directory) = self.directory {
            dl = dl.directory(directory);
        }
        if let Some(output) = self.output {
            dl = dl.output(output);
        }
        if let Some(query) = self.query {
            dl = dl.query(&query);
        }
        if self.interactive {
            dl = dl.interactive(false);
        } else if self.interactive_raw {
            dl = dl.interactive(true);
        }

        if self.list_streams {
            dl.list().await?;
        } else if self.parse {
            dl.parse().await?;
        } else {
            dl.download().await?;
        }
        Ok(())
    }
}
