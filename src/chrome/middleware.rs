use super::utils;
use anyhow::Result;
use headless_chrome::protocol::network::methods::GetResponseBodyReturnObject;
use kdam::term::Colorizer;
use std::io::Write;

fn decode(body: GetResponseBodyReturnObject) -> Result<Vec<u8>> {
    if body.base64_encoded {
        Ok(base64::decode(body.body)?)
    } else {
        Ok(body.body.as_bytes().to_vec())
    }
}

pub fn save_to_disk(url: &str, body: GetResponseBodyReturnObject, build: bool) -> Result<()> {
    let url = url.split('?').next().unwrap();

    if url.contains(".m3u") {
        let file = utils::filepath(&url, "m3u8");

        if build {
            utils::build_links(&decode(body)?, &file, url)?;
            println!(
                "Saved {} playlist from {} to {}",
                "BUILDED HLS".colorize("cyan"),
                url,
                file.colorize("bold green")
            );
        } else {
            std::fs::File::create(&file)?.write_all(&decode(body)?)?;
            println!(
                "Saved {} playlist from {} to {}",
                "HLS".colorize("cyan"),
                url,
                file.colorize("bold green")
            );
        }
    } else if url.contains(".mpd") {
        let file = utils::filepath(url, "mpd");
        std::fs::File::create(&file)?.write_all(&decode(body)?)?;
        println!(
            "Saved {} playlist from {} to {}",
            "DASH".colorize("cyan"),
            url,
            file.colorize("bold green")
        );
    } else if url.contains(".vtt") {
        let file = utils::filepath(url, "vtt");
        std::fs::File::create(&file)?.write_all(&decode(body)?)?;
        println!(
            "Saved {} from {} to {}",
            "SUBTITLES".colorize("cyan"),
            url,
            file.colorize("bold green")
        );
    } else if url.contains(".srt") {
        let file = utils::filepath(url, "srt");
        std::fs::File::create(&file)?.write_all(&decode(body)?)?;
        println!(
            "Saved {} from {} to {}",
            "SUBTITLES".colorize("cyan"),
            url,
            file.colorize("bold green")
        );
    }

    Ok(())
}
