use kdam::term::Colorizer;
use std::str::FromStr;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
pub enum Quality {
    yt_144p,
    yt_240p,
    yt_360p,
    yt_480p,
    yt_720p,
    yt_1080p,
    yt_2k,
    yt_1440p,
    yt_4k,
    yt_8k,
    Resolution(u16, u16),
    Highest,
    SelectLater,
}

impl FromStr for Quality {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "144p" => Self::yt_144p,
            "240p" => Self::yt_240p,
            "360p" => Self::yt_360p,
            "480p" => Self::yt_480p,
            "720p" | "hd" => Self::yt_720p,
            "1080p" | "fhd" => Self::yt_1080p,
            "2k" => Self::yt_2k,
            "1440p" | "qhd" => Self::yt_1440p,
            "4k" => Self::yt_4k,
            "8k" => Self::yt_8k,
            "highest" | "max" => Self::Highest,
            "select-later" => Self::SelectLater,
            x if x.contains("x") => {
                if let (Some(w), Some(h)) = (x.split("x").nth(0), x.split("x").nth(1)) {
                    Self::Resolution(
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
