use std::collections::HashMap;

use anyhow::{bail, Context, Result};

use crate::args::Quality;
use crate::utils::{format_bytes, select};

fn quality_selector(
    quality: &str,
    res_band: HashMap<&str, (usize, usize)>,
    master: &m3u8_rs::MasterPlaylist,
) -> Result<String> {
    if let Some(index) = res_band.get(quality) {
        println!("Selected variant stream at index {}", index.1);
        Ok(master.variants[index.1].uri.clone())
    } else {
        bail!(
            "Master playlist doesn't contain {} quality variant stream.",
            quality
        );
    }
}

pub fn master(
    master: &m3u8_rs::MasterPlaylist,
    quality: &Quality,
    raw_prompts: bool,
) -> Result<String> {
    let mut streams = vec![];
    let mut res_band: HashMap<&str, (usize, usize)> = HashMap::new();

    for (i, variant) in master.variants.iter().enumerate() {
        let bandwidth = variant.bandwidth.parse::<usize>().context(format!(
            "Couldn't parse bandwidth of variant playlist at index {}.",
            i
        ))?;
        let band_fmt = format_bytes(bandwidth);

        if let Some(resolution) = &variant.resolution {
            let res_fmt = match resolution.as_str() {
                "256x144" => "144p",
                "426x240" => "240p",
                "640x360" => "360p",
                "854x480" => "480p",
                "1280x720" => "720p",
                "1920x1080" => "1080p",
                "2560x1140" => "2K",
                "3840x2160" => "4K",
                _ => resolution.as_str(),
            };

            if let Some(pband) = res_band.get(res_fmt) {
                if bandwidth > pband.0 {
                    res_band.insert(res_fmt, (bandwidth, i));
                }
            } else {
                res_band.insert(res_fmt, (bandwidth, i));
            }

            streams.push(format!(
                "{:#2}) {:#9} {:>6} {}/s",
                i + 1,
                res_fmt,
                band_fmt.0,
                band_fmt.1,
            ));
        } else {
            streams.push(format!(
                "{:#2}) {:#9} {:>6} {}/s",
                i + 1,
                "?p",
                band_fmt.0,
                band_fmt.1,
            ));
        }
    }

    let uri = match quality {
        Quality::SD => quality_selector("480p", res_band, master)?,
        Quality::HD => quality_selector("720p", res_band, master)?,
        Quality::FHD => quality_selector("1080p", res_band, master)?,
        Quality::UHD => quality_selector("2K", res_band, master)?,
        Quality::UHD4K => quality_selector("4K", res_band, master)?,
        Quality::Select => {
            let index = if streams.len() == 1 {
                println!("Selected {} variant stream.", &streams[0]);
                0
            } else {
                select(
                    "Select one variant stream:".to_string(),
                    &streams,
                    raw_prompts,
                )?
            };

            master.variants[index].uri.clone()
        }

        Quality::Max => {
            let mut index = 0;
            let mut factor = 0;

            for (i, variant) in master.variants.iter().enumerate() {
                if let Some(resolution) = &variant.resolution {
                    let quality = resolution
                        .split('x')
                        .map(|x| {
                            x.parse::<usize>().unwrap_or_else(|_| panic!("Couldn't parse resolution of variant playlist at index {}.",
                                i))
                        })
                        .collect::<Vec<usize>>()
                        .iter()
                        .sum::<usize>()
                        + variant.bandwidth.parse::<usize>().context(format!(
                            "Couldn't parse bandwidth of variant playlist at index {}.",
                            i
                        ))?;

                    if quality > factor {
                        factor = quality;
                        index = i.to_owned();
                    }
                }
            }

            master.variants[index].uri.clone()
        }
    };

    Ok(uri)
}

pub fn alternative(master: &m3u8_rs::MasterPlaylist, raw_prompts: bool) -> Result<String> {
    let mut streams = vec![];

    for (i, alternative) in master.alternatives.iter().enumerate() {
        if alternative.uri.is_some() {
            let mut stream = format!(
                "{:#2}) {}: auto={}",
                i + 1,
                alternative.media_type,
                alternative.autoselect
            );

            if let Some(language) = &alternative.language {
                stream += &format!(" | language={}", language);
            }

            if let Some(channels) = &alternative.channels {
                stream += &format!(" | channels={}", channels);
            }

            streams.push(stream);
        }
    }

    let index = if streams.len() == 1 {
        println!("Selected {} alternative stream.", &streams[0]);
        0
    } else {
        select(
            "Select one alternative stream:".to_string(),
            &streams,
            raw_prompts,
        )?
    };

    Ok(master.alternatives[index].uri.clone().unwrap())
}
