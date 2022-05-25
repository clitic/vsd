use clap::{ArgEnum, Parser};

#[derive(Debug, Copy, Clone, ArgEnum)]
pub enum Quality {
    Select,
    SD,
    HD,
    FHD,
    UHD,
    UHD4K,
    Max,
}

/// Download HLS video from a website, m3u8 url or from a local m3u8 file.
#[derive(Debug, Clone, Parser)]
#[clap(version, author = "clitic <clitic21@gmail.com>", about)]
pub struct Args {
    /// url | .m3u8 | .m3u
    #[clap(required = true, validator = input_validator)]
    pub input: String,

    /// Path for output of downloaded video stream.
    #[clap(short, long)]
    pub output: Option<String>,

    /// Automatic selection of some standard resolution streams with highest bandwidth stream variant.
    #[clap(short, long, arg_enum, default_value_t = Quality::Select)]
    pub quality: Quality,

    /// Base url for all segments, usally needed for local m3u8 file.
    #[clap(short, long)]
    pub baseurl: Option<String>,

    /// Maximum number of threads for parllel downloading of segments.
    #[clap(short, long, default_value_t = 5, validator = threads_validator)]
    pub threads: u8,

    /// Custom request headers for sending to streaming server. 
    #[clap(long, multiple_occurrences = true, number_of_values = 2, value_names = &["key", "value"])]
    pub header: Vec<String>, // Vec<Vec<String>> not supported

    /// Update and set custom user agent for requests.
    #[clap(
        long,
        default_value = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.64 Safari/537.36"
    )]
    pub user_agent: String,

    /// Custom http or https proxy address for requests.
    #[clap(long, validator = proxy_address_validator)]
    pub proxy_address: Option<String>,

    /// Maximum number of retries to download an individual segment.
    #[clap(long, default_value_t = 10)]
    pub retry_count: u8,

    /// Launch Google Chrome to capture requests specific to HLS and Dash related contents.
    #[clap(long)]
    pub capture: bool,

    /// Launch Google Chrome without a window for interaction.
    #[clap(long)]
    pub headless: bool,
	
	// delete temporary downloaded segments, add --no-cleanup flag to use resume capabilities
    //#[clap(short, long)]
    //pub resume: bool,
	
	// path of ffmpeg binary
    //#[clap(long)]
    //pub ffmpeg: Option<String>,
}

fn input_validator(s: &str) -> Result<(), String> {
    if !s.starts_with("http") {
        println!("Non HTTP input should have `--baseurl` set explicitly");
    }

    Ok(())
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

pub fn parse() -> Args {
    Args::parse()
}
