use anyhow::{bail, Result};
use clap::Args;
use kdam::term::Colorizer;
use reqwest::blocking::Client;
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Proxy, Url};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Download HLS and Dash playlists.
///
/// Playlists which have separate audio tracks or subtitles streams, expects
/// ffmpeg (https://www.ffmpeg.org/download.html) to be installed in system PATH.
#[derive(Debug, Clone, Args)]
pub struct Save {
    /// URL | .m3u8 | .m3u | .mpd | .xml
    #[arg(required = true)]
    pub input: String,

    /// Path of final downloaded video stream.
    /// For file extension any ffmpeg supported format could be provided.
    /// If playlist contains alternative streams vsd will try to transmux and trancode into single file using ffmpeg.
    #[arg(short, long)]
    pub output: Option<String>,

    /// Base url for all segments.
    /// Usually needed for local m3u8 file.
    #[arg(short, long)]
    pub baseurl: Option<String>,

    /// Automatic selection of some standard resolution streams with highest bandwidth stream variant from master playlist.
    /// possible values: [144p, 240p, 360p, 480p, 720p, hd, 1080p, fhd, 2k, 1440p, qhd, 4k, 8k, highest, max, select-later]
    #[arg(short, long, default_value = "select-later", value_name = "WIDTHxHEIGHT", value_parser = quality_parser)]
    pub quality: Quality,

    // /// Automatic selection of some standard resolution streams with highest bandwidth stream variant from master playlist.
    // #[arg(long, number_of_values = 2, value_names = &["width", "height"])]
    // pub resolution: Vec<u64>,
    /// Maximum number of threads for parllel downloading of segments.
    /// Number of threads should be in range 1-16 (inclusive).
    #[arg(short, long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=16))]
    pub threads: u8,

    /// Maximum number of retries to download an individual segment.
    #[arg(long, default_value_t = 15)]
    pub retry_count: u8,

    /// Raw style input prompts for old and unsupported terminals.
    #[arg(long)]
    pub raw_prompts: bool,

    /// Resume a download session.
    /// Download session can only be resumed if download session json file is present.
    #[arg(short, long)]
    pub resume: bool,

    /// Download alternative streams such as audio and subtitles streams from master playlist instead of variant video streams.
    #[arg(short, long)]
    pub alternative: bool,

    /// Skip downloading and muxing alternative streams.
    #[arg(short, long)]
    pub skip: bool,

    /// TODO: Decryption keys.
    /// This option can be used multiple times.
    #[arg(short, long, value_name = "<KID:KEY>|KEY")]
    pub key: Vec<String>,

    /// TODO: Record duration for live playlist in seconds.
    #[arg(long)]
    pub record_duration: Option<f32>,

    /// TODO: Directory path
    #[arg(long)]
    pub save_directory: Option<String>,

    /// Custom headers for requests.
    /// This option can be used multiple times.
    #[arg(long, number_of_values = 2, value_names = &["KEY", "VALUE"], help_heading = "Client Options")]
    pub header: Vec<String>, // Vec<Vec<String>> not supported

    /// Update and set custom user agent for requests.
    #[arg(
        long,
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.64 Safari/537.36",
        help_heading = "Client Options"
    )]
    pub user_agent: String,

    /// Set http or https proxy address for requests.
    #[arg(long, value_parser = proxy_address_parser, help_heading = "Client Options")]
    pub proxy_address: Option<String>,

    /// Enable cookie store which allows cookies to be stored.
    #[arg(long, help_heading = "Client Options")]
    pub enable_cookies: bool,

    /// Enable cookie store and fill it with some existing cookies.
    /// Example `--cookies "foo=bar; Domain=yolo.local" https://yolo.local`.
    /// This option can be used multiple times.
    #[arg(long, number_of_values = 2, value_names = &["COOKIES", "URL"], help_heading = "Client Options")]
    pub cookies: Vec<String>, // Vec<Vec<String>> not supported
}

#[derive(Debug, Clone)]
pub enum Quality {
    Youtube144p,
    Youtube240p,
    Youtube360p,
    Youtube480p,
    Youtube720p,
    Youtube1080p,
    Youtube2k,
    Youtube1440p,
    Youtube4k,
    Youtube8k,
    Resolution(u16, u16),
    Highest,
    SelectLater,
}

fn quality_parser(s: &str) -> Result<Quality, String> {
    Ok(match s.to_lowercase().as_str() {
        "144p" => Quality::Youtube144p,
        "240p" => Quality::Youtube240p,
        "360p" => Quality::Youtube360p,
        "480p" => Quality::Youtube480p,
        "720p" | "hd" => Quality::Youtube720p,
        "1080p" | "fhd" => Quality::Youtube1080p,
        "2k" => Quality::Youtube2k,
        "1440p" | "qhd" => Quality::Youtube1440p,
        "4k" => Quality::Youtube4k,
        "8k" => Quality::Youtube8k,
        "highest" | "max" => Quality::Highest,
        "select-later" => Quality::SelectLater,
        x if x.contains('x') => {
            if let (Some(w), Some(h)) = (x.split('x').next(), x.split('x').nth(1)) {
                Quality::Resolution(
                    w.parse::<u16>().map_err(|_| "invalid width".to_owned())?,
                    h.parse::<u16>().map_err(|_| "invalid height".to_owned())?,
                )
            } else {
                Err("incorrect resolution format".to_owned())?
            }
        }
        _ => Err(format!(
            "\npossible values: [{}]\nFor custom resolution use {}",
            [
                "144p",
                "240p",
                "360p",
                "480p",
                "720p",
                "hd",
                "1080p",
                "fhd",
                "2k",
                "1440p",
                "qhd",
                "4k",
                "8k",
                "highest",
                "max",
                "select-later",
            ]
            .iter()
            .map(|x| x.colorize("green"))
            .collect::<Vec<_>>()
            .join(", "),
            "WIDTHxHEIGHT".colorize("green")
        ))?,
    })
}

fn proxy_address_parser(s: &str) -> Result<String, String> {
    if s.starts_with("http://") || s.starts_with("https://") {
        Ok(s.to_owned())
    } else {
        Err("Proxy address should start with `http://` or `https://` only".to_string())
    }
}

impl Save {
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
        let output = self
            .input
            .split('?')
            .next()
            .unwrap()
            .split('/')
            .last()
            .unwrap();

        let output = if output.ends_with(".m3u") || output.ends_with(".m3u8") {
            if output.ends_with(".ts.m3u8") {
                output.trim_end_matches(".m3u8").to_owned()
            } else {
                let mut path = PathBuf::from(&output);
                path.set_extension("ts");
                path.to_str().unwrap().to_owned()
            }
        } else if output.ends_with(".mpd") || output.ends_with(".xml") {
            let mut path = PathBuf::from(&output);
            path.set_extension("m4s");
            path.to_str().unwrap().to_owned()
        } else {
            let mut path = PathBuf::from(
                output
                    .replace('<', "-")
                    .replace('>', "-")
                    .replace(':', "-")
                    .replace('\"', "-")
                    .replace('/', "-")
                    .replace('\\', "-")
                    .replace('|', "-")
                    .replace('?', "-"),
            );
            path.set_extension("mp4");
            path.to_str().unwrap().to_owned()
        };

        if Path::new(&output).exists() && !self.resume {
            let stemed_path = Path::new(&output).file_stem().unwrap().to_str().unwrap();
            let ext = Path::new(&output).extension().unwrap().to_str().unwrap();

            for i in 1.. {
                let core_file_copy = format!("{} ({}).{}", stemed_path, i, ext);

                if !Path::new(&core_file_copy).exists() {
                    return core_file_copy;
                }
            }
        }

        output
    }

    pub fn input_type(&self) -> InputType {
        let url = self.input.split('?').next().unwrap();

        if url.starts_with("http") {
            if url.ends_with(".m3u") || url.ends_with(".m3u8") || url.ends_with("ts.m3u8") {
                InputType::HlsUrl
            } else if url.ends_with(".mpd") || url.ends_with(".xml") {
                InputType::DashUrl
            } else {
                InputType::Website
            }
        } else if url.ends_with(".m3u") || url.ends_with(".m3u8") || url.ends_with("ts.m3u8") {
            InputType::HlsLocalFile
        } else if url.ends_with(".mpd") || url.ends_with(".xml") {
            InputType::DashLocalFile
        } else {
            InputType::LocalFile
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
