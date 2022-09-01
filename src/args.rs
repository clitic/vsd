use crate::utils;
use anyhow::{bail, Result};
use clap::{ArgEnum, ArgGroup, Parser};
use kdam::term::Colorizer;
use reqwest::blocking::Client;
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Proxy, Url};
use std::path::Path;
use std::sync::Arc;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, ArgEnum)]
pub enum Quality {
    yt_144p,
    yt_240p,
    yt_360p,
    yt_480p,
    HD,
    FHD,
    FHD_2K,
    QHD,
    UHD_4K,
    FUHD_8K,
    Highest,
    Select,
}

fn input_validator(s: &str) -> Result<(), String> {
    if s.starts_with("https://youtube.com")
        || s.starts_with("https://www.youtube.com")
        || s.starts_with("https://youtu.be")
    {
        Err("Youtube links aren't supported yet".to_owned())
    } else {
        Ok(())
    }
}

fn threads_validator(s: &str) -> Result<(), String> {
    let num_threads: usize = s.parse().map_err(|_| format!("`{}` isn't a number", s))?;
    if std::ops::RangeInclusive::new(1, 16).contains(&num_threads) {
        Ok(())
    } else {
        Err("Number of threads should be in range `1-16`".to_string())
    }
}

fn proxy_address_validator(s: &str) -> Result<(), String> {
    if s.starts_with("http://") || s.starts_with("https://") {
        Ok(())
    } else {
        Err("Proxy address should start with `http://` or `https://` only".to_string())
    }
}

/// Command line program to download HLS video from websites and m3u8 links.
#[derive(Debug, Clone, Parser)]
#[clap(version, author = "clitic <clitic21@gmail.com>", about, group = ArgGroup::new("chrome").args(&["capture", "collect"]))]
pub struct Args {
    /// URL | .m3u8 | .m3u | .mpd | .xml
    #[clap(required = true, validator = input_validator)]
    pub input: String,

    /// Path of final downloaded video stream.
    /// For file extension any ffmpeg supported format could be provided.
    /// If playlist contains alternative streams vsd will try to transmux and trancode into single file using ffmpeg.
    #[clap(short, long)]
    pub output: Option<String>,

    /// Base url for all segments.
    /// Usually needed for local m3u8 file.
    #[clap(short, long)]
    pub baseurl: Option<String>,

    /// Automatic selection of some standard resolution streams with highest bandwidth stream variant from master playlist.
    /// yt prefixed qualities are qualities used by youtube.
    #[clap(short, long, arg_enum, default_value_t = Quality::Select)]
    pub quality: Quality,

    // /// Automatic selection of some standard resolution streams with highest bandwidth stream variant from master playlist.
    // #[clap(long, number_of_values = 2, value_names = &["width", "height"])]
    // pub resolution: Vec<u64>,

    // parse manuaal
    // pub resolution: Vec<u64>,
    /// Maximum number of threads for parllel downloading of segments.
    /// Number of threads should be in range 1-16 (inclusive).
    #[clap(short, long, default_value_t = 5, validator = threads_validator)]
    pub threads: u8,

    /// Maximum number of retries to download an individual segment.
    #[clap(long, default_value_t = 15)]
    pub retry_count: u8,

    /// Raw style input prompts for old and unsupported terminals.
    #[clap(long)]
    pub raw_prompts: bool,

    /// Resume a download session.
    /// Download session can only be resumed if download session json file is present.
    #[clap(short, long)]
    pub resume: bool,

    /// Download alternative streams such as audio and subtitles streams from master playlist instead of variant video streams.
    #[clap(short, long)]
    pub alternative: bool,

    /// Skip downloading and muxing alternative streams.
    #[clap(short, long)]
    pub skip: bool,

    /// TODO Decryption keys.
    /// This option can be used multiple times.
    #[clap(short, long, multiple_occurrences = true, value_name = "<KID:KEY>|KEY")]
    pub key: Vec<String>,

    /// Launch Google Chrome to capture requests made to fetch .m3u8 (HLS) and .mpd (Dash) files.
    #[clap(long, help_heading = "CHROME OPTIONS")]
    pub capture: bool,

    /// Launch Google Chrome and collect .m3u8 (HLS), .mpd (Dash) and subtitles from a website and save them locally.
    #[clap(long, help_heading = "CHROME OPTIONS")]
    pub collect: bool,

    /// Launch Google Chrome without a window for interaction.
    /// This option should must be used with `--capture` or `--collect` flag only.
    #[clap(long, requires = "chrome", help_heading = "CHROME OPTIONS")]
    pub headless: bool,

    /// Build http links for all uri present in .m3u8 file while collecting it.
    /// Resultant .m3u8 file can be played and downloaded directly without the need of `--baseurl` flag.
    /// This option should must be used with `--collect` flag only.
    #[clap(long, requires = "collect", help_heading = "CHROME OPTIONS")]
    pub build: bool,

    /// Custom headers for requests.
    /// This option can be used multiple times.
    #[clap(long, multiple_occurrences = true, number_of_values = 2, value_names = &["KEY", "VALUE"], help_heading = "CLIENT OPTIONS")]
    pub header: Vec<String>, // Vec<Vec<String>> not supported

    /// Update and set custom user agent for requests.
    #[clap(
        long,
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.64 Safari/537.36",
        help_heading = "CLIENT OPTIONS"
    )]
    pub user_agent: String,

    /// Set http or https proxy address for requests.
    #[clap(long, validator = proxy_address_validator, help_heading = "CLIENT OPTIONS")]
    pub proxy_address: Option<String>,

    /// Enable cookie store which allows cookies to be stored.
    #[clap(long, help_heading = "CLIENT OPTIONS")]
    pub enable_cookies: bool,

    /// Enable cookie store and fill it with some existing cookies.
    /// Example `--cookies "foo=bar; Domain=yolo.local" https://yolo.local`.
    /// This option can be used multiple times.
    #[clap(long, multiple_occurrences = true, number_of_values = 2, value_names = &["COOKIES", "URL"], help_heading = "CLIENT OPTIONS")]
    pub cookies: Vec<String>, // Vec<Vec<String>> not supported
}

impl Args {
    pub fn client(&self) -> Result<Arc<Client>> {
        let mut client_builder = Client::builder().user_agent(&self.user_agent);

        if !self.header.is_empty() {
            let mut headers = HeaderMap::new();

            for i in (0..headers.len()).step_by(2) {
                headers.insert(
                    self.header[i].parse::<HeaderName>()?,
                    self.header[i + 1].parse::<HeaderValue>()?,
                );
            }

            client_builder = client_builder.default_headers(headers);
        }

        if let Some(proxy) = &self.proxy_address {
            if proxy.starts_with("https") {
                client_builder = client_builder.proxy(Proxy::https(proxy)?);
            } else if proxy.starts_with("http") {
                client_builder = client_builder.proxy(Proxy::http(proxy)?);
            }
        }

        if self.enable_cookies || !self.cookies.is_empty() {
            client_builder = client_builder.cookie_store(true);
        }

        if !self.cookies.is_empty() {
            let jar = Jar::default();

            for i in (0..self.cookies.len()).step_by(2) {
                jar.add_cookie_str(&self.cookies[i], &self.cookies[i + 1].parse::<Url>()?);
            }

            client_builder = client_builder.cookie_provider(Arc::new(jar));
        }

        Ok(Arc::new(client_builder.build()?))
    }

    pub fn get_url(&self, uri: &str) -> Result<String> {
        if uri.starts_with("http") {
            Ok(uri.to_owned())
        } else if let Some(baseurl) = &self.baseurl {
            Ok(Url::parse(baseurl)?.join(uri)?.to_string())
        } else {
            if !self.input.starts_with("http") {
                bail!(
                    "Non HTTP input should have {} set explicitly.",
                    "--baseurl".colorize("bold green")
                )
            }

            Ok(Url::parse(&self.input)?.join(uri)?.to_string())
        }
    }

    pub fn tempfile(&self) -> String {
        let path = if let Some(output) = self
            .input
            .split('?')
            .next()
            .unwrap()
            .split('/')
            .find(|x| x.ends_with(".m3u8"))
        {
            if output.ends_with(".ts.m3u8") {
                output.trim_end_matches(".m3u8").to_owned()
            } else {
                utils::replace_ext(output, "ts")
            }
        } else {
            "merged.ts".to_owned()
        };

        if Path::new(&path).exists() && !self.resume {
            let stemed_path = Path::new(&path).file_stem().unwrap().to_str().unwrap();

            for i in 1..9999 {
                let core_file_copy = format!("{} ({}).ts", stemed_path, i);

                if !Path::new(&core_file_copy).exists() {
                    return core_file_copy;
                }
            }
        }
        path
    }

    pub fn input_type(&self) -> InputType {
        let url = self.input.split('?').next().unwrap();

        if url.starts_with("http") {
            if url.ends_with(".m3u8") || url.ends_with(".m3u") || url.ends_with("ts.m3u8") {
                InputType::HlsUrl
            } else if url.ends_with(".mpd") || url.ends_with(".xml") {
                InputType::DashUrl
            } else {
                InputType::Website
            }
        } else {
            if url.ends_with(".m3u8") || url.ends_with(".m3u") || url.ends_with("ts.m3u8") {
                InputType::HlsLocalFile
            } else if url.ends_with(".mpd") || url.ends_with(".xml") {
                InputType::DashLocalFile
            } else {
                InputType::LocalFile
            }
        }
    }
}
pub enum InputType {
    HlsUrl,
    DashUrl,
    Website,
    HlsLocalFile,
    DashLocalFile,
    LocalFile,
}

impl InputType {
    pub fn is_website(&self) -> bool {
        match &self {
            Self::Website => true,
            _ => false,
        }
    }

    pub fn is_hls(&self) -> bool {
        match &self {
            Self::HlsUrl | Self::HlsLocalFile => true,
            _ => false,
        }
    }

    pub fn is_dash(&self) -> bool {
        match &self {
            Self::DashUrl | Self::DashLocalFile => true,
            _ => false,
        }
    }
}
