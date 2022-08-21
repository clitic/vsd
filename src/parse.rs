use anyhow::{bail, Result};
use crate::args::Quality;
use crate::utils::{format_bytes, select};

fn resolution(res: &m3u8_rs::Resolution) -> String {
    match (res.width, res.height) {
        (256, 144) => "144p".to_owned(),
        (426, 240) => "240p".to_owned(),
        (640, 360) => "360p".to_owned(),
        (854, 480) => "480p".to_owned(),
        (1280, 720) => "720p".to_owned(),
        (1920, 1080) => "1080p".to_owned(),
        (2048, 1080) => "2K".to_owned(),
        (2560, 1440) => "1440p".to_owned(),
        (3840, 2160) => "4K".to_owned(),
        (7680, 4320) => "8K".to_owned(),
        (w, h) => format!("{}x{}", w, h),
    }
}

fn quality_selector(
    quality: &str,
    sorted_variants: Vec<&m3u8_rs::VariantStream>,
) -> Result<String> {
    if let Some(variant) = sorted_variants.iter().find(|x| {
        if let Some(res) = x.resolution {
            return quality == resolution(&res);
        }

        false
    }) {
        let band_fmt = format_bytes(variant.bandwidth as usize);
        println!("Selected variant stream of quality {} ({} {}/s)", quality, band_fmt.0, band_fmt.1);
        Ok(variant.uri.clone())
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

    let sorted_variants = {
        let mut sorted_variants = vec![];

        for variant in master.variants.iter() {
            if let Some(resolution) = &variant.resolution {
                let quality = resolution.width + resolution.height;
                sorted_variants.push((quality, variant));
            } else {
                sorted_variants.push((0, variant));
            }
        }

        sorted_variants.sort_by(|x, y| {
            if (y.0 > x.0) || (y.0 == x.0 && y.1.bandwidth > x.1.bandwidth) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        });

        sorted_variants.iter().map(|x| x.1).collect::<Vec<_>>()
    };

    for (i, variant) in sorted_variants.iter().enumerate() {
        let band_fmt = format_bytes(variant.bandwidth as usize);

        if let Some(resolution) = &variant.resolution {
            let res_fmt = match (resolution.width, resolution.height) {
                (256, 144) => "144p".to_owned(),
                (426, 240) => "240p".to_owned(),
                (640, 360) => "360p".to_owned(),
                (854, 480) => "480p".to_owned(),
                (1280, 720) => "720p".to_owned(),
                (1920, 1080) => "1080p".to_owned(),
                (2048, 1080) => "2K".to_owned(),
                (2560, 1440) => "1440p".to_owned(),
                (3840, 2160) => "4K".to_owned(),
                (7680, 4320) => "8K".to_owned(),
                (w, h) => format!("{}x{}", w, h),
            };

            streams.push(format!(
                "{:2}) {:9} {:>6} {}/s",
                i + 1,
                res_fmt,
                band_fmt.0,
                band_fmt.1,
            ));
        } else {
            streams.push(format!(
                "{:2}) {:9} {:>6} {}/s",
                i + 1,
                "?p",
                band_fmt.0,
                band_fmt.1,
            ));
        }
    }

    let uri = match quality {
        Quality::yt_144p => quality_selector("144p", sorted_variants)?,
        Quality::yt_240p => quality_selector("240p", sorted_variants)?,
        Quality::yt_360p => quality_selector("360p", sorted_variants)?,
        Quality::yt_480p => quality_selector("480p", sorted_variants)?,
        Quality::HD => quality_selector("720p", sorted_variants)?,
        Quality::FHD => quality_selector("1080p", sorted_variants)?,
        Quality::FHD_2K => quality_selector("2K", sorted_variants)?,
        Quality::QHD => quality_selector("1440p", sorted_variants)?,
        Quality::UHD_4K => quality_selector("4K", sorted_variants)?,
        Quality::FUHD_8K => quality_selector("8K", sorted_variants)?,
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

            sorted_variants[index].uri.clone()
        }

        Quality::Highest => sorted_variants[0].uri.clone(),
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
                stream += " | language={}";
                stream += language;
            }

            if let Some(channels) = &alternative.channels {
                stream += " | channels={}";
                stream += channels;
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
