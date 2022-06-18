use std::io::Write;

use anyhow::{anyhow, bail, Result};
use headless_chrome::browser::tab::RequestInterceptionDecision;
use headless_chrome::protocol::network::methods::RequestPattern;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use kdam::term::Colorizer;

fn filepath(url: &str, ext: &str) -> String {
    let path = if let Some(output) = url.split("/").find(|x| x.ends_with(ext)) {
        crate::utils::replace_ext(output.split("?").next().unwrap(), ext)
    } else {
        format!("playlist.{}", ext)
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

pub fn capture(url: String, headless: bool) -> Result<()> {
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
    tab.navigate_to(url.as_str())
        .map_err(|e| anyhow!(e.to_string()))?;

    tab.enable_request_interception(
        &[RequestPattern {
            url_pattern: None,
            resource_type: Some("XHR"),
            interception_stage: None,
        }],
        Box::new(|_transport, _session_id, intercepted| {
            if intercepted.request.url.contains(".m3u") || intercepted.request.url.contains(".mpd")
            {
                println!("• {}", intercepted.request.url);
            }

            RequestInterceptionDecision::Continue
        }),
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    std::thread::sleep(std::time::Duration::from_secs(60 * 3));
    Ok(())
}

pub fn collect(
    url: String,
    headless: bool,
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
    tab.navigate_to(url.as_str())
        .map_err(|e| anyhow!(e.to_string()))?;

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

            if url.starts_with("https://cache-video.iq.com/dash")
                || (url.contains(".m3u") || url.contains(".mpd"))
                || url.contains(".vtt")
            {
                sender.lock().unwrap().send(url).unwrap();
            }

            RequestInterceptionDecision::Continue
        }),
    )
    .map_err(|e| anyhow!(e.to_string()))?;

    if url.starts_with("https://www.iq.com/play") {
        println!(
            "Collection method available for {}\nUsing {} method for collection.",
            "https://www.iq.com/play".colorize("bold green"),
            "custom".colorize("cyan")
        );
    } else {
        println!("Using {} method for collection.", "comman".colorize("cyan"))
    }

    println!(
        "Terminate this program using {}",
        "CTRL+C".colorize("bold red")
    );
    while let Ok(xhr_url) = receiver.recv() {
        if xhr_url.starts_with("https://cache-video.iq.com/dash") {
            iqiyi(&url, &xhr_url, &downloader)?;
        } else if xhr_url.contains(".m3u") {
            let file = filepath(&xhr_url, "m3u8");
            std::fs::File::create(&file)?.write(&downloader.get_bytes(&xhr_url)?)?;
            println!(
                "Saved {} playlist from {} to {}",
                "HLS".colorize("cyan"),
                xhr_url,
                file.colorize("bold green")
            );
        } else if xhr_url.contains(".mpd") {
            let file = filepath(&xhr_url, "m3u8");
            std::fs::File::create(&file)?.write(&downloader.get_bytes(&xhr_url)?)?;
            println!(
                "Saved {} playlist from {} to {}",
                "DASH".colorize("cyan"),
                xhr_url,
                file.colorize("bold green")
            );
        } else if xhr_url.contains(".vtt") {
            let file = filepath(&xhr_url, "vtt");
            std::fs::File::create(&file)?.write(&downloader.get_bytes(&xhr_url)?)?;
            println!(
                "Saved {} to {}",
                "subtitles".colorize("cyan"),
                file.colorize("bold green")
            );
        }
    }

    Ok(())
}

fn iqiyi(url: &str, xhr_url: &str, downloader: &crate::downloader::Downloader) -> Result<()> {
    if !url.starts_with("https://www.iq.com/play") {
        bail!("Only https://www.iq.com/play links are supported.")
    }

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

    let v: serde_json::Value = serde_json::from_str(&downloader.get(&xhr_url)?.text()?)?;

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
