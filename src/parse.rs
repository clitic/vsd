use crate::args::Quality;
use crate::utils::{format_bytes, select};
use anyhow::{bail, Result};
use std::fmt::Write;

fn select_quality(quality: &str, variants: Vec<&m3u8_rs::VariantStream>) -> Result<String> {
    if let Some(variant) = variants
        .iter()
        .find(|x| quality == resolution(x.resolution))
    {
        let band_fmt = format_bytes(variant.bandwidth as usize);
        println!(
            "Selected variant stream of quality {} ({} {}/s).",
            quality, band_fmt.0, band_fmt.1
        );
        return Ok(variant.uri.clone());
    }

    bail!(
        "Master playlist doesn't contain {} quality variant stream.",
        quality
    )
}

fn resolution(res: Option<m3u8_rs::Resolution>) -> String {
    if let Some(res) = res {
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
    } else {
        "?".to_owned()
    }
}

pub fn master(
    master: &m3u8_rs::MasterPlaylist,
    quality: &Quality,
    raw_prompts: bool,
) -> Result<String> {
    let variants = {
        let mut sorted_variants = vec![];

        for variant in master.variants.iter().filter(|x| !x.is_i_frame) {
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

    if variants.len() == 1 {
        let band_fmt = format_bytes(variants[0].bandwidth as usize);
        println!(
            "Only one variant stream found.\nSelected variant stream of quality {} ({} {}/s).",
            resolution(variants[0].resolution),
            band_fmt.0,
            band_fmt.1
        );

        return Ok(variants[0].uri.clone());
    }


    let uri = match quality {
        Quality::yt_144p => select_quality("144p", variants)?,
        Quality::yt_240p => select_quality("240p", variants)?,
        Quality::yt_360p => select_quality("360p", variants)?,
        Quality::yt_480p => select_quality("480p", variants)?,
        Quality::HD => select_quality("720p", variants)?,
        Quality::FHD => select_quality("1080p", variants)?,
        Quality::FHD_2K => select_quality("2K", variants)?,
        Quality::QHD => select_quality("1440p", variants)?,
        Quality::UHD_4K => select_quality("4K", variants)?,
        Quality::FUHD_8K => select_quality("8K", variants)?,
        Quality::Highest => variants[0].uri.clone(),
        Quality::Select => {
            let mut streams = vec![];
            for (i, variant) in variants.iter().enumerate() {
                let band_fmt = format_bytes(variant.bandwidth as usize);

                streams.push(format!(
                    "{:2}) {:9} {:>6} {}/s",
                    i + 1,
                    resolution(variant.resolution),
                    band_fmt.0,
                    band_fmt.1,
                ));
            }

            let index = select(
                "Select one variant stream:".to_string(),
                &streams,
                raw_prompts,
            )?;

            variants[index].uri.clone()
        }
    };

    Ok(uri)
}

pub fn alternative(master: &m3u8_rs::MasterPlaylist, raw_prompts: bool) -> Result<String> {
    let mut streams = vec![];

    for (i, alternative) in master.alternatives.iter().enumerate() {
        if alternative.uri.is_some() {
            let mut stream = format!(
                "{:#2}) {}: autoselect ({})",
                i + 1,
                alternative.media_type,
                alternative.autoselect
            );

            if let Some(language) = &alternative.language {
                let _ = write!(stream, ", language ({})", language);
            }

            if let Some(channels) = &alternative.channels {
                let _ = write!(stream, ", channels ({})", channels);
            }

            streams.push(stream);
        }
    }

    if streams.len() == 0 {
        let index = select(
            "Select one alternative stream:".to_string(),
            &streams,
            raw_prompts,
        )?;
    
        Ok(master.alternatives[index].uri.clone().unwrap())
    } else {
        bail!("No alternative streams found in master playlist.")
    }
}
