use crate::{downloader::Prompts, playlist::PlaylistType};
use anyhow::{anyhow, bail, Result};
use kdam::term::Colorizer;
use regex::Regex;
use reqwest::{blocking::Client, header, Url};
use std::{collections::HashSet, io::Write, path::Path};

pub struct InputMetadata {
    pub pl_type: Option<PlaylistType>,
    pub text: String,
    pub url: Url,
}

impl InputMetadata {
    fn fetch(&mut self, client: &Client) -> Result<()> {
        let response = client.get(self.url.as_ref()).send()?;
        self.url = response.url().to_owned();

        if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
            match content_type.as_bytes() {
                b"application/dash+xml" | b"video/vnd.mpeg.dash.mpd" => {
                    self.pl_type = Some(PlaylistType::Dash)
                }
                b"application/x-mpegurl" | b"application/vnd.apple.mpegurl" => {
                    self.pl_type = Some(PlaylistType::Hls)
                }
                _ => (),
            }
        }

        self.text = response.text()?;
        self.update_pl_type_from_text();
        Ok(())
    }

    fn update_pl_type_from_text(&mut self) {
        if self.pl_type.is_none() {
            if self.text.contains("<MPD") {
                self.pl_type = Some(PlaylistType::Dash);
            } else if self.text.contains("#EXTM3U") {
                self.pl_type = Some(PlaylistType::Hls);
            }
        }
    }
}

pub fn fetch_playlist(
    base_url: Option<Url>,
    client: &Client,
    input: &str,
    prompts: &Prompts,
) -> Result<InputMetadata> {
    let mut meta = InputMetadata {
        pl_type: None,
        text: String::new(),
        url: base_url
            .clone()
            .unwrap_or_else(|| "https://example.com".parse::<Url>().unwrap()),
    };
    let path = Path::new(input);

    if path.exists() {
        if base_url.is_none() {
            println!(
                "    {} base url is not set",
                "Warning".colorize("bold yellow")
            );
        }

        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy();
            if ext == "mpd" {
                meta.pl_type = Some(PlaylistType::Dash);
            } else if ext == "m3u" || ext == "m3u8" {
                meta.pl_type = Some(PlaylistType::Hls);
            }
        }

        meta.text = std::fs::read_to_string(path)?;
        meta.update_pl_type_from_text();
    } else {
        meta.url = input.parse::<Url>().unwrap();
        // TODO - We can add site specific parsers here
        meta.fetch(client)?;

        if meta.pl_type.is_none() {
            fetch_from_website(client, &mut meta, prompts)?;
        }
    }

    Ok(meta)
}

fn fetch_from_website(client: &Client, meta: &mut InputMetadata, prompts: &Prompts) -> Result<()> {
    println!(
        "   {} website for DASH and HLS playlists",
        "Scraping".colorize("bold cyan")
    );

    let links = scrape_playlist_links(&meta.text);

    match links.len() {
        0 => bail!("No playlists were found in website source."),
        1 => {
            println!("      {} {}", "Found".colorize("bold green"), &links[0]);
            meta.url = links[0].parse::<Url>()?;
        }
        _ => {
            if prompts.skip || prompts.raw {
                println!("Select one playlist:");

                for (i, link) in links.iter().enumerate() {
                    println!("{:2}) [{}] {}", i + 1, if i == 0 { 'x' } else { ' ' }, link);
                }

                println!("------------------------------");

                let mut index = 0;

                if prompts.raw && !prompts.skip {
                    print!(
                        "Press enter to proceed with defaults.\n\
                    Or select playlist to download (1, 2, etc.): "
                    );
                    std::io::stdout().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;

                    println!("------------------------------");

                    let input = input.trim();

                    if !input.is_empty() {
                        index = input
                            .parse::<usize>()
                            .map_err(|_| anyhow!("input is not a valid positive number."))?
                            - 1;
                    }
                }

                meta.url = links
                    .get(index)
                    .ok_or_else(|| anyhow!("selected playlist is out of index bounds."))?
                    .parse::<Url>()?;
                println!("   {} {}", "Selected".colorize("bold green"), meta.url);
            } else {
                let question = requestty::Question::select("scraped-link")
                    .message("Select one playlist")
                    .should_loop(false)
                    .choices(links)
                    .build();
                let answer = requestty::prompt_one(question)?;
                meta.url = answer.as_list_item().unwrap().text.parse::<Url>()?;
            }
        }
    }

    meta.fetch(client)?;
    Ok(())
}

fn scrape_playlist_links(text: &str) -> Vec<String> {
    let re =
        Regex::new(r#"([\"\'])(https?:\/\/[^\"\']*\.(m3u8|m3u|mpd)[^\"\']*)([\"\'])"#).unwrap();
    let links = re
        .captures_iter(text)
        .map(|caps| caps.get(2).unwrap().as_str().to_string())
        .collect::<HashSet<String>>();

    // in case of amalgated urls
    links
        .into_iter()
        .map(|x| {
            if x.starts_with("http") {
                if let Some(y) = x.split("http").last() {
                    return format!("http{}", y);
                }
            }
            x
        })
        .collect()
}
