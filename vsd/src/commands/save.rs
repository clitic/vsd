use crate::{
    cookie::Cookies,
    downloader::{self, MAX_RETRIES, MAX_THREADS, SKIP_DECRYPT, SKIP_MERGE},
    options::Interaction,
};
use anyhow::{Result, bail};
use clap::Args;
use reqwest::{
    Client, Proxy, Url,
    cookie::Jar,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::fs;

/// Download DASH and HLS playlists.
#[derive(Args, Clone, Debug)]
pub struct Save {
    /// http(s):// | .mpd | .xml | .m3u8
    #[arg(required = true)]
    pub input: String,

    /// Base url to be used for building absolute url to segment.
    /// This flag is usually needed for local input files.
    /// By default redirected playlist url is used.
    #[arg(long)]
    pub base_url: Option<Url>,

    /// Change directory path for temporarily downloaded files.
    /// By default current working directory is used.
    #[arg(short, long)]
    pub directory: Option<PathBuf>,

    /// Mux all downloaded streams to a video container (.mp4, .mkv, etc.) using ffmpeg.
    /// Note that existing files will be overwritten and downloaded streams will be deleted.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Parse playlist and returns it in json format.
    /// Note that --output flag is ignored when this flag is used.
    #[arg(long)]
    pub parse: bool,

    /// Force some specific subtitle codec when muxing through ffmpeg.
    /// By default `mov_text` is used for .mp4 and `copy` for others.
    #[arg(long, default_value = "copy")]
    pub subs_codec: String,

    /// Prompt for custom streams selection with modern style input prompts. By default proceed with defaults.
    #[arg(
        short,
        long,
        help_heading = "Automation Options",
        conflicts_with = "interactive_raw"
    )]
    pub interactive: bool,

    /// Prompt for custom streams selection with raw style input prompts. By default proceed with defaults.
    #[arg(short = 'I', long, help_heading = "Automation Options")]
    pub interactive_raw: bool,

    /// List all the streams present inside the playlist.
    #[arg(short, long, help_heading = "Automation Options")]
    pub list_streams: bool,

    /// Filters to be applied for automatic stream selection.
    #[arg(
        short,
        long,
        help_heading = "Automation Options",
        default_value = "v=best:s=en",
        long_help = "Filters to be applied for automatic stream selection.\n\n\
        SYNTAX: `v={}:a={}:s={}` where `{}` (in priority order) can contain\n\
        |> all: select all streams.\n\
        |> skip: skip all streams or select inverter.\n\
        |> 1,2: indices obtained by --list-streams flag.\n\
        |> 1080p,1280x720: stream resolution.\n\
        |> en,fr: stream language.\n\n\
        EXAMPLES:\n\
        |> v=skip:a=skip:s=all (download all sub streams)\n\
        |> a:en:s=en (prefer en lang)\n\
        |> v=1080p:a=all:s=skip (1080p with all audio streams)\n"
    )]
    pub select_streams: String,

    /// Fill request client with some existing cookies value.
    /// It should be a path to a file containing cookies in netscape format.
    #[arg(long, help_heading = "Client Options")]
    pub cookies: Option<PathBuf>,

    /// Extra headers for requests in same format as curl.
    ///
    /// This option can be used multiple times.
    #[arg(short = 'H', long = "header", help_heading = "Client Options", value_name = "KEY:VALUE", value_parser = Self::parse_header)]
    pub headers: Vec<(HeaderName, HeaderValue)>,

    /// Skip checking and validation of site certificates.
    #[arg(long, help_heading = "Client Options")]
    pub no_certificate_checks: bool,

    /// Set http(s) / socks proxy address for requests.
    #[arg(long, help_heading = "Client Options", value_parser = Self::parse_proxy)]
    pub proxy: Option<Proxy>,

    /// Set query parameters for requests.
    #[arg(long, help_heading = "Client Options", default_value = "", hide_default_value = true, value_parser = Self::parse_query)]
    pub query: HashMap<String, String>,

    /// Keys for decrypting encrypted streams.
    /// KID:KEY should be specified in hex format.
    #[arg(long, help_heading = "Decrypt Options", value_name = "KID:KEY;…", default_value = "", hide_default_value = true, value_parser = Self::parse_keys)]
    pub keys: HashMap<String, String>,

    /// Download encrypted streams without decrypting them.
    /// Note that --output flag is ignored if this flag is used.
    #[arg(long, help_heading = "Decrypt Options")]
    pub no_decrypt: bool,

    /// Download streams without merging them.
    /// Note that --output flag is ignored if this flag is used.
    #[arg(long, help_heading = "Download Options")]
    pub no_merge: bool,

    /// Maximum number of retries to download an individual segment.
    #[arg(long, help_heading = "Download Options", default_value_t = 10)]
    pub retries: u8,

    /// Total number of threads for parllel downloading of segments.
    /// Number of threads should be in range 1-16 (inclusive).
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

    fn parse_query(s: &str) -> Result<HashMap<String, String>> {
        let mut queries = HashMap::new();

        if s.is_empty() {
            return Ok(queries);
        }

        for pair in s.split('&') {
            let mut parts = pair.splitn(2, '=');
            if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                queries.insert(key.to_owned(), value.to_owned());
            }
        }

        Ok(queries)
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
        MAX_RETRIES.store(self.retries, Ordering::SeqCst);
        MAX_THREADS.store(self.threads, Ordering::SeqCst);
        SKIP_DECRYPT.store(self.no_decrypt, Ordering::SeqCst);
        SKIP_MERGE.store(self.no_merge, Ordering::SeqCst);

        let mut builder = Client::builder()
            .default_headers(HeaderMap::from_iter(self.headers))
            .cookie_store(true)
            .danger_accept_invalid_certs(self.no_certificate_checks)
            .timeout(Duration::from_secs(60));

        if let Some(path) = &self.cookies {
            let jar = Jar::default();
            let data = fs::read(path).await?;

            for cookie in Cookies::parse(&data)?.0 {
                jar.add_cookie_str(&cookie.to_header(), &cookie.url().parse::<Url>()?);
            }

            builder = builder.cookie_provider(Arc::new(jar));
        }

        if let Some(proxy) = &self.proxy {
            builder = builder.proxy(proxy.clone());
        }

        let client = builder.build()?;
        let meta =
            downloader::fetch_playlist(self.base_url.clone(), &client, &self.input, &self.query)
                .await?;

        if self.list_streams {
            downloader::list_all_streams(&meta)?;
        } else if self.parse {
            let playlist =
                downloader::parse_all_streams(self.base_url.clone(), &client, &meta, &self.query)
                    .await?;
            serde_json::to_writer(std::io::stdout(), &playlist)?;
        } else {
            let streams = downloader::parse_selected_streams(
                self.base_url.clone(),
                &client,
                &meta,
                &self.query,
                self.select_streams.parse().unwrap(),
                match (self.interactive, self.interactive_raw) {
                    (true, false) => Interaction::Modern,
                    (false, true) => Interaction::Raw,
                    _ => Interaction::None,
                },
            )
            .await?;

            downloader::download(
                self.base_url,
                client,
                self.directory,
                self.keys,
                self.output,
                self.query,
                streams,
                self.subs_codec,
            )
            .await?;
        }

        Ok(())
    }
}
