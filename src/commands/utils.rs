#![cfg(feature = "chrome")]

use anyhow::{anyhow, Result};
use kdam::term::Colorizer;
use std::path::{Path, PathBuf};

pub(super) fn chrome_launch_message(headless: bool) {
    println!(
        "Chrome will launch {} a window for 3 minutes.",
        if headless { "without" } else { "with" },
    );
}

#[cfg(feature = "chrome")]
pub(super) fn chrome_warning_message() {
    println!(
        "Sometimes video starts playing but links are not captured \
		if such condition occurs then try re running the command.\n\
        Terminate this program using {}",
        "CTRL+C".colorize("bold red")
    );
}

pub(super) fn filepath(url: &str, ext: &str) -> String {
    let path = if let Some(output) = url
        .split('?')
        .next()
        .unwrap()
        .split('/')
        .find(|x| x.ends_with(&format!(".{}", ext)))
    {
        if output.ends_with(&format!(".ts.{}", ext)) {
            let mut path = PathBuf::from(output.trim_end_matches(&format!(".{}", ext)));
            path.set_extension(ext);
            path.to_str().unwrap().to_owned()
        } else {
            let mut path = PathBuf::from(output);
            path.set_extension(ext);
            path.to_str().unwrap().to_owned()
        }
    } else {
        match ext {
            "m3u8" => "playlist.m3u8".to_owned(),
            "mpd" => "playlist.mpd".to_owned(),
            "vtt" => "subtitles.vtt".to_owned(),
            "srt" => "subtitles.srt".to_owned(),
            _ => format!("unknown.{}", ext),
        }
    };

    if Path::new(&path).exists() {
        let stemed_path = Path::new(&path).file_stem().unwrap().to_str().unwrap();

        for i in 1.. {
            let core_file_copy = format!("{}_({}).{}", stemed_path, i, ext);

            if !Path::new(&core_file_copy).exists() {
                return core_file_copy;
            }
        }
    }

    path
}

pub(super) fn build_links(m3u8: &[u8], file: &str, baseurl: &str) -> Result<()> {
    match m3u8_rs::parse_playlist_res(m3u8)
        .map_err(|_| anyhow!("Couldn't parse {} playlist.", baseurl))?
    {
        m3u8_rs::Playlist::MasterPlaylist(master) => {
            let mut master_c = master;

            for variant in master_c.variants.iter_mut() {
                if !variant.uri.starts_with("http") {
                    variant.uri = reqwest::Url::parse(baseurl)?
                        .join(&variant.uri)?
                        .to_string();
                }
            }

            for alternative in master_c.alternatives.iter_mut() {
                if let Some(uri) = &alternative.uri {
                    if !uri.starts_with("http") {
                        alternative.uri =
                            Some(reqwest::Url::parse(baseurl)?.join(uri)?.to_string());
                    }
                }
            }

            master_c.write_to(&mut std::fs::File::create(file)?)?;
        }
        m3u8_rs::Playlist::MediaPlaylist(meadia) => {
            let mut meadia_c = meadia;

            for segment in meadia_c.segments.iter_mut() {
                if !segment.uri.starts_with("http") {
                    segment.uri = reqwest::Url::parse(baseurl)?
                        .join(&segment.uri)?
                        .to_string();
                }
            }

            meadia_c.write_to(&mut std::fs::File::create(file)?)?;
        }
    }

    Ok(())
}
