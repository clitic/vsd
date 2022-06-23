use std::io::Write;

use anyhow::{anyhow, Result};
use headless_chrome::browser::tab::RequestInterceptionDecision;
use headless_chrome::protocol::network::methods::RequestPattern;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use kdam::term::Colorizer;

pub fn message(headless: bool) {
    println!("Some websites use window size to check wheter to show quality switch button or not. \
        For such websites open chrome in full-screen mode and then right-click and select inspect. \
        Now resize the window as required.\n\
        {}\n\
        Chrome will launch {} a window.\n\
        Terminate this program using {}\n",
        "Sometimes request interception doesn't works in such condition try re running the command."
        .colorize("#FFA500"), 
        if headless {
            "without"
        } else {
            "with"
        }, "CTRL+C".colorize("bold red")
    );
}

fn filepath(url: &str, ext: &str) -> String {
    let path = if let Some(output) = url
        .split("?")
        .next()
        .unwrap()
        .split("/")
        .find(|x| x.ends_with(&format!(".{}", ext)))
    {
        if output.ends_with(&format!(".ts.{}", ext)) {
            crate::utils::replace_ext(output.trim_end_matches(&format!(".{}", ext)), ext)
        } else {
            crate::utils::replace_ext(output, ext)
        }
    } else {
        match ext {
            "m3u8" => "playlist.m3u8".to_owned(),
            "mpd" => "manifest.mpd".to_owned(),
            "vtt" => "subtitles.vtt".to_owned(),
            "srt" => "subtitles.srt".to_owned(),
            _ => format!("unknown.{}", ext),
        }
    };

    if std::path::Path::new(&path).exists() {
        let stemed_path = std::path::Path::new(&path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();

        for i in 1..9999 {
            let core_file_copy = format!("{} ({}).{}", stemed_path, i, ext);

            if !std::path::Path::new(&core_file_copy).exists() {
                return core_file_copy;
            }
        }
    }
    path
}

pub fn capture(url: &str, headless: bool) -> Result<()> {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(headless)
            .build()
            .map_err(|e| anyhow!(e))?,
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    let tab = browser
        .wait_for_initial_tab()
        .map_err(|e| anyhow!(e.to_string()))?;
    tab.navigate_to(url).map_err(|e| anyhow!(e.to_string()))?;

    let count = std::sync::atomic::AtomicU8::new(1);

    tab.enable_request_interception(
        &[RequestPattern {
            url_pattern: None,
            resource_type: Some("XHR"),
            interception_stage: None,
        }],
        Box::new(move |_transport, _session_id, intercepted| {
            if intercepted.request.url.contains(".m3u") || intercepted.request.url.contains(".mpd")
            {
                let i = count.load(std::sync::atomic::Ordering::SeqCst);
                println!("{}) {}", i, intercepted.request.url);
                count.store(i + 1, std::sync::atomic::Ordering::SeqCst);
            }

            RequestInterceptionDecision::Continue
        }),
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    std::thread::sleep(std::time::Duration::from_secs(60 * 3));
    Ok(())
}

pub fn collect(
    url: &str,
    headless: bool,
    build: bool,
    downloader: &crate::downloader::Downloader,
) -> Result<()> {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(headless)
            .build()
            .map_err(|e| anyhow!(e))?,
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    let tab = browser
        .wait_for_initial_tab()
        .map_err(|e| anyhow!(e.to_string()))?;
    tab.navigate_to(url).map_err(|e| anyhow!(e.to_string()))?;

    let (sender, receiver) = std::sync::mpsc::channel();
    let sender = std::sync::Mutex::new(sender);

    tab.enable_request_interception(
        &[RequestPattern {
            url_pattern: None,
            resource_type: Some("XHR"),
            interception_stage: None,
        }],
        Box::new(move |_transport, _session_id, intercepted| {
            let url = intercepted.request.url;

            if url.contains(".m3u")
                || url.contains(".mpd")
                || url.contains(".vtt")
                || url.contains(".srt")
                || url.starts_with("https://cache-video.iq.com/dash")
            {
                sender.lock().unwrap().send(url).unwrap();
            }

            RequestInterceptionDecision::Continue
        }),
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    if url.starts_with("https://www.iq.com/play") {
        println!("Using {} method for collection.", "CUSTOM".colorize("cyan"));
    } else {
        println!("Using {} method for collection.", "COMMAN".colorize("cyan"))
    }

    while let Ok(xhr_url) = receiver.recv() {
        if xhr_url.contains(".m3u") {
            let file = filepath(&xhr_url, "m3u8");

            if build {
                build_links(&xhr_url, &file, &downloader)?;
                println!(
                    "Saved {} playlist from {} to {}",
                    "BUILDED HLS".colorize("cyan"),
                    xhr_url,
                    file.colorize("bold green")
                );
            } else {
                downloader.write_to_file(&xhr_url, &file)?;
                println!(
                    "Saved {} playlist from {} to {}",
                    "HLS".colorize("cyan"),
                    xhr_url,
                    file.colorize("bold green")
                );
            }
        } else if xhr_url.contains(".mpd") {
            let file = filepath(&xhr_url, "mpd");
            downloader.write_to_file(&xhr_url, &file)?;
            println!(
                "Saved {} playlist from {} to {}",
                "DASH".colorize("cyan"),
                xhr_url,
                file.colorize("bold green")
            );
        } else if xhr_url.contains(".vtt") {
            let file = filepath(&xhr_url, "vtt");
            downloader.write_to_file(&xhr_url, &file)?;
            println!(
                "Saved {} from {} to {}",
                "SUBTITLES".colorize("cyan"),
                xhr_url,
                file.colorize("bold green")
            );
        } else if xhr_url.contains(".srt") {
            let file = filepath(&xhr_url, "srt");
            downloader.write_to_file(&xhr_url, &file)?;
            println!(
                "Saved {} from {} to {}",
                "SUBTITLES".colorize("cyan"),
                xhr_url,
                file.colorize("bold green")
            );
        } else if xhr_url.starts_with("https://cache-video.iq.com/dash") {
            iqiyi(&url, &xhr_url, &downloader)?;
        }
    }

    Ok(())
}

fn build_links(
    xhr_url: &str,
    file: &str,
    downloader: &crate::downloader::Downloader,
) -> Result<()> {
    match m3u8_rs::parse_playlist_res(&downloader.get_bytes(&xhr_url)?)
        .map_err(|_| anyhow!("Couldn't parse {} playlist.", xhr_url))?
    {
        m3u8_rs::Playlist::MasterPlaylist(master) => {
            let mut master_c = master.clone();

            for variant in master_c.variants.iter_mut() {
                if !variant.uri.starts_with("http") {
                    variant.uri = reqwest::Url::parse(&xhr_url)?
                        .join(&variant.uri)?
                        .to_string();
                }
            }

            for alternative in master_c.alternatives.iter_mut() {
                if let Some(uri) = &alternative.uri {
                    if !uri.starts_with("http") {
                        alternative.uri =
                            Some(reqwest::Url::parse(&xhr_url)?.join(uri)?.to_string());
                    }
                }
            }

            master_c.write_to(&mut std::fs::File::create(&file)?)?;
        }
        m3u8_rs::Playlist::MediaPlaylist(meadia) => {
            let mut meadia_c = meadia.clone();

            for segment in meadia_c.segments.iter_mut() {
                if !segment.uri.starts_with("http") {
                    segment.uri = reqwest::Url::parse(&xhr_url)?
                        .join(&segment.uri)?
                        .to_string();
                }
            }

            meadia_c.write_to(&mut std::fs::File::create(&file)?)?;
        }
    }

    Ok(())
}

fn iqiyi(url: &str, xhr_url: &str, downloader: &crate::downloader::Downloader) -> Result<()> {
    let re = regex::Regex::new(r"[a-zA-Z0-9-]*\?").unwrap();
    let name = re
        .captures_iter(&url)
        .next()
        .unwrap()
        .get(0)
        .unwrap()
        .as_str()
        .trim_end_matches("?")
        .to_owned();

    let v = downloader.get_json(&xhr_url)?;

    // Here unwrap method is used intentionally.
    for video in v["data"]["program"]["video"].as_array().unwrap() {
        if video["_selected"] == serde_json::json!(true) {
            let resolution = video["scrsz"].as_str().unwrap();
            let content = video["m3u8"].as_str().unwrap();
            let file = format!("{}_{}_video.m3u8", name, resolution);

            if !std::path::Path::new(&file).exists() {
                std::fs::File::create(&file)?.write(content.as_bytes())?;
                println!(
                    "Saved {} playlist to {}",
                    "HLS".colorize("cyan"),
                    file.colorize("bold green")
                );
            }
        }
    }

    for subtitles in v["data"]["program"]["stl"].as_array().unwrap() {
        let url = format!(
            "https://meta.video.iqiyi.com{}",
            subtitles["srt"].as_str().unwrap()
        );
        let language = subtitles["_name"].as_str().unwrap();
        let file = format!("{}_{}_subtitles.srt", name, language).replace(" ", "_");

        if !std::path::Path::new(&file).exists() {
            std::fs::File::create(&file)?.write(&downloader.get_bytes(&url)?)?;
            println!(
                "Saved {} subtitles to {}",
                language.colorize("cyan"),
                file.colorize("bold green")
            );
        }
    }

    Ok(())
}
