use super::utils;
use anyhow::Result;
use headless_chrome::browser::tab::RequestInterceptionDecision;
use headless_chrome::protocol::network::events::RequestInterceptedEventParams;
use kdam::term::Colorizer;
use reqwest::blocking::Client;
use std::sync::Arc;

pub fn intercept(
    params: RequestInterceptedEventParams,
    client: Arc<Client>,
    build: bool,
) -> Result<RequestInterceptionDecision> {
    let url = params.request.url;

    // println!(
    //     "{}",
    //     "-".repeat(crate::utils::get_columns() as usize)
    //         .colorize("#FFA500")
    // );

    if url.contains(".m3u") {
        let file = utils::filepath(&url, "m3u8");

        if build {
            let m3u8 = utils::write_to_file(&client, params.request.headers, &url, &file)?;
            utils::build_links(m3u8.as_bytes(), &file, &url)?;
            println!(
                "Saved {} playlist from {} to {}",
                "BUILDED HLS".colorize("cyan"),
                url,
                file.colorize("bold green")
            );
        } else {
            let _ = utils::write_to_file(&client, params.request.headers, &url, &file)?;
            println!(
                "Saved {} playlist from {} to {}",
                "HLS".colorize("cyan"),
                url,
                file.colorize("bold green")
            );
        }
    } else if url.contains(".mpd") {
        let file = utils::filepath(&url, "mpd");
        let _ = utils::write_to_file(&client, params.request.headers, &url, &file)?;
        println!(
            "Saved {} playlist from {} to {}",
            "DASH".colorize("cyan"),
            url,
            file.colorize("bold green")
        );
    } else if url.contains(".vtt") {
        let file = utils::filepath(&url, "vtt");
        let _ = utils::write_to_file(&client, params.request.headers, &url, &file)?;
        println!(
            "Saved {} from {} to {}",
            "SUBTITLES".colorize("cyan"),
            url,
            file.colorize("bold green")
        );
    } else if url.contains(".srt") {
        let file = utils::filepath(&url, "srt");
        let _ = utils::write_to_file(&client, params.request.headers, &url, &file)?;
        println!(
            "Saved {} from {} to {}",
            "SUBTITLES".colorize("cyan"),
            url,
            file.colorize("bold green")
        );
    }

    Ok(RequestInterceptionDecision::Continue)
}
