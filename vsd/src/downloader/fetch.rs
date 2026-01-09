use crate::{automation::Prompter, playlist::PlaylistType};
use anyhow::{Result, anyhow, bail};
use colored::Colorize;
use regex::Regex;
use reqwest::{Url, Client, header};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::Path,
};

pub struct Metadata {
    pub pl_type: Option<PlaylistType>,
    pub text: String,
    pub url: Url,
}

impl Metadata {
    async fn fetch(&mut self, client: &Client, query: &HashMap<String, String>) -> Result<()> {
        let response = client.get(self.url.as_ref()).query(query).send().await?;
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

        self.text = response.text().await?;
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

pub async fn fetch_playlist(
    base_url: Option<Url>,
    client: &Client,
    input: &str,
    prompter: &Prompter,
    query: &HashMap<String, String>,
) -> Result<Metadata> {
    let mut meta = Metadata {
        pl_type: None,
        text: String::new(),
        url: input
            .parse::<Url>()
            .unwrap_or("https://example.com".parse::<Url>().unwrap()),
    };
    let path = Path::new(input);

    if path.exists() {
        if base_url.is_none() {
            println!("    {} base url is not set", "Warning".yellow());
        }

        if let Some(ext) = path.extension() {
            if ext == "mpd" {
                meta.pl_type = Some(PlaylistType::Dash);
            } else if ext == "m3u" || ext == "m3u8" {
                meta.pl_type = Some(PlaylistType::Hls);
            }
        }

        meta.text = fs::read_to_string(path)?;
        meta.update_pl_type_from_text();
    } else {
        // TODO - We can add site specific parsers here
        meta.fetch(client, query).await?;

        if meta.pl_type.is_none() {
            fetch_from_website(client, &mut meta, prompter, query).await?;
        }
    }

    Ok(meta)
}

async fn fetch_from_website(
    client: &Client,
    meta: &mut Metadata,
    prompter: &Prompter,
    query: &HashMap<String, String>,
) -> Result<()> {
    println!(
        "   {} [generic-regex] website for DASH and HLS playlists",
        "Scraping".bold().cyan()
    );

    let links = scrape_playlist_links(&meta.text);

    match links.len() {
        0 => bail!("no playlists were found in website source."),
        1 => {
            println!("            {}", &links[0]);
            println!("   {} {}", "Selected".bold().green(), &links[0]);
            meta.url = links[0].parse::<Url>()?;
        }
        _ => {
            if prompter.interactive {
                let question = requestty::Question::select("scraped-link")
                    .message("Select one playlist")
                    .should_loop(false)
                    .choices(links)
                    .build();
                let answer = requestty::prompt_one(question)?;
                meta.url = answer.as_list_item().unwrap().text.parse::<Url>()?;
            } else if prompter.interactive_raw {
                println!("Select one playlist:");

                for (i, link) in links.iter().enumerate() {
                    println!(
                        "{:2}) [{}] {}",
                        i + 1,
                        if i == 0 {
                            "x".green()
                        } else {
                            " ".normal()
                        },
                        link
                    );
                }

                println!("{}", "------------------------------".cyan());
                print!(
                    "Press enter to proceed with defaults.\n\
                    Or select playlist to download (1, 2, etc.): "
                );

                std::io::stdout().flush()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;

                println!("{}", "------------------------------".cyan());

                let input = input.trim();
                let mut index = 0;

                if !input.is_empty() {
                    index = input
                        .parse::<usize>()
                        .map_err(|_| anyhow!("input is not a valid positive number."))?
                        - 1;
                }

                meta.url = links
                    .get(index)
                    .ok_or_else(|| anyhow!("selected playlist is out of index bounds."))?
                    .parse::<Url>()?;
                println!("   {} {}", "Selected".bold().green(), meta.url);
            } else {
                for link in &links {
                    println!("            {link}");
                }

                println!("   {} {}", "Selected".bold().green(), &links[0]);
                meta.url = links[0].parse::<Url>()?;
            }
        }
    }

    meta.fetch(client, query).await?;
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
            if x.starts_with("http")
                && let Some(y) = x.split("http").last() {
                    return format!("http{y}");
                }
            x
        })
        .collect()
}
