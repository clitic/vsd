use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use vsd;

use kdam::term::Colorizer;

pub struct DownloadState {
    args: vsd::args::Args,
    downloader: vsd::downloader::Downloader,
    audio_stream: bool,
    subtitle_stream: bool,
}

impl DownloadState {
    pub fn new() -> Result<Self> {
        let args = vsd::args::parse();

        if args.capture {
            println!(
                "Opening chrome in headless={} mode for 3 minutes.",
                args.headless
            );
            vsd::capture::run(args.input, args.headless);
            std::process::exit(0);
        }

        let downloader = vsd::downloader::Downloader::new_custom(
            args.user_agent.clone(),
            args.header.clone(),
            args.proxy_address.clone(),
        )
        .context("Failed to create reqwest client.")?;

        if vsd::utils::find_ffmpeg_with_path().is_none() {
            println!(
                "{} FFMPEG couldn't be located\n{:#4}{} {}",
                "[✗]".colorize("bold red"),
                "",
                "✗".colorize("red"),
                "https://www.ffmpeg.org/download.html"
            );
        }

        // let temp_file = if args.output.ends_with(".srt") {
        //     vsd::path::replace_ext(&args.output, "vtt")
        // } else {
        //     vsd::path::replace_ext(&args.output, "ts")
        // };

        Ok(Self {
            args,
            downloader,
            audio_stream: false,
            subtitle_stream: false,
        })
    }

    pub fn get_url(&self, uri: &str) -> Result<String> {
        if uri.starts_with("http") {
            Ok(uri.to_owned())
        } else {
            if let Some(baseurl) = &self.args.baseurl {
                Ok(vsd::utils::join_url(baseurl, &uri)?)
            } else {
                Ok(vsd::utils::join_url(&self.args.input, &uri)?)
            }
        }
    }

    pub fn scrape_webpage(&mut self) -> Result<()> {
        if !self.args.input.ends_with(".m3u8") || self.args.input.ends_with(".html") {
            println!(
                "Input was found to be a webpage. Proceeding to scrape it for HLS and Dash links."
            );

            let resp = self
                .downloader
                .get(self.args.input.clone())
                .context("Failed to scrape webpage. Make sure you are connected to internet.")?;

            if resp.status() == reqwest::StatusCode::OK {
                let links = vsd::utils::find_hls_dash_links(resp.text().unwrap());

                match links.len() {
                    0 => bail!(
                        "No HLS and Dash links found on webpage. Consider using `--capture` flag."
                    ),
                    1 => {
                        self.args.input = links[0].clone();
                        println!("Only one link found {}", &links[0]);
                    }
                    _ => {
                        let mut elinks = vec![];
                        for (i, link) in links.iter().enumerate() {
                            elinks.push(format!("{:#2}) {}", i, link));
                        }
                        let index =
                            vsd::utils::select_index("Select one link:".to_string(), elinks)?;
                        self.args.input = links[index].clone();
                    }
                }

                println!("Input was switched to {}", self.args.input);
            } else {
                bail!(
                    "{} returned HTTP status code {}",
                    self.args.input,
                    resp.status()
                );
            }
        }

        Ok(())
    }

    fn quality_selector(
        &self,
        quality: &str,
        resolution_bandwidth_map: HashMap<&str, (usize, usize)>,
        master: &m3u8_rs::MasterPlaylist,
    ) -> Result<String> {
        if let Some(index) = resolution_bandwidth_map.get(quality) {
            Ok(master.variants[index.1].uri.clone())
        } else {
            bail!(
                "Master playlist doesn't contain {} quality variant stream.",
                quality
            );
        }
    }

    pub fn parse_master(&self, master: &m3u8_rs::MasterPlaylist) -> Result<String> {
        let mut streams = vec![];
        let mut rbmap: HashMap<&str, (usize, usize)> = HashMap::new();

        for (i, variant) in master.variants.iter().enumerate() {
            let bandwidth = variant.bandwidth.parse::<usize>().context(format!(
                "Failed to parse bandwidth of variant playlist at index {}.",
                i
            ))?;
            let bandwidth_fmt = vsd::utils::format_bytes(bandwidth);

            if let Some(resolution) = &variant.resolution {
                let resolution_fmt = match resolution.as_str() {
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

                if let Some(pbandwidth) = rbmap.get(resolution_fmt) {
                    if bandwidth > pbandwidth.0 {
                        rbmap.insert(resolution_fmt, (bandwidth, i));
                    }
                } else {
                    rbmap.insert(resolution_fmt, (bandwidth, i));
                }

                streams.push(format!(
                    "{:#2}) {:#9} {:>6} {}/s",
                    i + 1,
                    resolution_fmt,
                    bandwidth_fmt.0,
                    bandwidth_fmt.1,
                ));
            } else {
                streams.push(format!(
                    "{:#2}) {:#9} {:>6} {}/s",
                    i + 1,
                    "?p",
                    bandwidth_fmt.0,
                    bandwidth_fmt.1,
                ));
            }
        }

        let uri = match self.args.quality {
            vsd::args::Quality::Select => {
                let index =
                    vsd::utils::select_index("Select one variant stream:".to_string(), streams)?;
                master.variants[index].uri.clone()
            }

            vsd::args::Quality::Max => {
                let mut index = 0;
                let mut factor = 0;

                for (i, variant) in master.variants.iter().enumerate() {
                    if let Some(resolution) = &variant.resolution {
                        let quality = resolution
                            .split("x")
                            .map(|x| {
                                x.parse::<usize>().expect(&format!(
                                    "Failed to parse resolution of variant playlist at index {}.",
                                    i
                                ))
                            })
                            .collect::<Vec<usize>>()
                            .iter()
                            .sum::<usize>()
                            + variant.bandwidth.parse::<usize>().context(format!(
                                "Failed to parse bandwidth of variant playlist at index {}.",
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
            vsd::args::Quality::SD => self.quality_selector("480p", rbmap, &master)?,
            vsd::args::Quality::HD => self.quality_selector("720p", rbmap, &master)?,
            vsd::args::Quality::FHD => self.quality_selector("1080p", rbmap, &master)?,
            vsd::args::Quality::UHD => self.quality_selector("2K", rbmap, &master)?,
            vsd::args::Quality::UHD4K => self.quality_selector("4K", rbmap, &master)?,
        };

        Ok(self.get_url(&uri)?)
    }

    pub fn parse_master_alternative(&mut self, master: &m3u8_rs::MasterPlaylist) -> Result<()> {
        let mut audio_stream = false;
        let mut subtitle_stream = false;

        for alternative in &master.alternatives {
            match alternative.media_type {
                m3u8_rs::AlternativeMediaType::Audio => {
                    if alternative.autoselect {
                        if let Some(uri) = &alternative.uri {
                            println!("Re-targeting to download audio stream.");

                            let args = self.args.clone();
                            self.args.input = self.get_url(uri).unwrap();
                            self.args.output = None;

                            let content =
                                self.downloader.get_bytes(self.args.input.clone()).unwrap();
                            match m3u8_rs::parse_playlist_res(&content).unwrap() {
                                m3u8_rs::Playlist::MediaPlaylist(meadia) => {
                                    self.download(&meadia.segments, format!("{}_audio.ts", self.determine_output().trim_end_matches(".ts")))
                                        ?;
                                }
                                _ => (),
                            }

                            audio_stream = true;
                            self.args = args;
                        }
                    }
                }

                m3u8_rs::AlternativeMediaType::Subtitles
                | m3u8_rs::AlternativeMediaType::ClosedCaptions => {
                    if alternative.autoselect {
                        if let Some(uri) = &alternative.uri {
                            println!("Re-targeting to download subtitle stream.");

                            let args = self.args.clone();
                            self.args.input = self.get_url(uri).unwrap();
                            self.args.output = Some(format!("{}_subtitles.srt", self.determine_output().trim_end_matches(".ts")));

                            let content =
                                self.downloader.get_bytes(self.args.input.clone()).unwrap();
                            match m3u8_rs::parse_playlist_res(&content).unwrap() {
                                m3u8_rs::Playlist::MediaPlaylist(meadia) => {
                                    self.download(&meadia.segments, format!("{}_subtitles.vtt", self.determine_output().trim_end_matches(".ts")))
                                        ?;
                                }
                                _ => (),
                            }

                            subtitle_stream = true;
                            self.args = args;
                        }
                    }
                }

                _ => (),
            }
        }

        self.audio_stream = audio_stream;
        self.subtitle_stream = subtitle_stream;
        Ok(())
    }

    pub fn segments(&mut self) -> Result<Vec<m3u8_rs::MediaSegment>> {
        let content = if self.args.input.starts_with("http") {
            self.downloader.get_bytes(self.args.input.clone())?
        } else {
            std::fs::read_to_string(self.args.input.clone())
                .context(format!("Failed to read `{}`", self.args.input))?
                .as_bytes()
                .to_vec()
        };

        match m3u8_rs::parse_playlist_res(&content).unwrap() {
            m3u8_rs::Playlist::MasterPlaylist(master) => {
                self.args.input = self.parse_master(&master)?;
                println!("Input was switched to {}", self.args.input);

                self.parse_master_alternative(&master)?;

                let playlist = self.downloader.get_bytes(self.args.input.clone()).unwrap();
                match m3u8_rs::parse_playlist_res(&playlist).unwrap() {
                    m3u8_rs::Playlist::MediaPlaylist(meadia) => {
                        return Ok(meadia.segments);
                    }
                    _ => bail!("Media playlist not found."),
                }
            }
            m3u8_rs::Playlist::MediaPlaylist(meadia) => {
                return Ok(meadia.segments);
            }
        }
    }

    pub fn determine_output(&self) -> String {
        let path = if let Some(output) = self.args.input.split("/").find(|x| x.ends_with(".m3u8")) {
            vsd::path::replace_ext(output, "ts")
        } else {
            "merged.ts".to_owned()
        };

        if std::path::Path::new(&path).exists() {
            let stemed_path = std::path::Path::new(&path)
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap();

            for i in 1..100 {
                let core_file_copy = format!("{} ({}).ts", stemed_path, i);

                if !std::path::Path::new(&core_file_copy).exists() {
                    return core_file_copy;
                }
            }
        }

        path
    }

    pub fn download(&self, segments: &Vec<m3u8_rs::MediaSegment>, tempfile: String) -> Result<()> {
        println!("Temporary file will be saved at {}", tempfile);

        let total = segments.len();

        let pb = Arc::new(Mutex::new(kdam::tqdm!(
            total = total,
            unit = "ts".to_owned()
        )));
        pb.lock().unwrap().refresh();

        let merger = Arc::new(Mutex::new(vsd::merger::BinarySequence::new(
            total,
            tempfile.clone(),
        )));

        let client = Arc::new(self.downloader.clone());
        let pool = threadpool::ThreadPool::new(self.args.threads as usize);

        for (i, segment) in segments.iter().enumerate() {
            let pb = pb.clone();
            let merger = merger.clone();
            let client = client.clone();
            let uri = self.get_url(&segment.uri)?;
            let total_retries = self.args.retry_count.clone();
            let mut retries = 0;
            let byterange = segment.byte_range.clone();
            let key = segment.key.clone();

            let key_uri = match &segment.key {
                Some(m3u8_rs::Key {
                    uri: Some(link), ..
                }) => Some(self.get_url(&link)?),
                _ => None,
            };

            pool.execute(move || {
                let mut data = loop {
                    let resp = match byterange {
                        Some(m3u8_rs::ByteRange {
                            length: start,
                            offset: Some(end),
                        }) => client.get_bytes_range(uri.clone(), start, start + end - 1),
                        _ => client.get_bytes(uri.clone()),
                    };

                    if resp.is_ok() {
                        break resp.unwrap();
                    } else {
                        if total_retries > retries {
                            pb.lock().unwrap().write(format!(
                                "{} {}",
                                "Retrying:".colorize("bold yellow"),
                                uri
                            ));
                            retries += 1;
                            continue;
                        } else {
                            pb.lock().unwrap().write(format!(
                                "{} Reached maximum number of retries for {}",
                                "Error:".colorize("bold red"),
                                uri
                            ));
                            std::process::exit(1);
                        }
                    }
                };

                if let Some(eku) = key_uri {
                    data = vsd::decrypt::HlsDecrypt::from_key(
                        key.unwrap(),
                        client.get_bytes(eku).unwrap(),
                    )
                    .decrypt(&data);
                }

                let mut merger = merger.lock().unwrap();
                merger.write(i, &data).unwrap();
                merger.flush().unwrap();

                let mut pb = pb.lock().unwrap();

                pb.set_description(format!(
                    "{} / {}",
                    vsd::utils::format_bytes_joined(merger.stored()),
                    vsd::utils::format_bytes_joined(merger.estimate())
                ));

                pb.update(1);
            });
        }

        pool.join();
        eprint!("\n");
        merger.lock().unwrap().flush().unwrap();

        if merger.lock().unwrap().buffered() {
            println!("File {} downloaded successfully.", tempfile);
        } else {
            bail!("File {} is not downloaded successfully.", tempfile);
        }

        if let Some(output) = &self.args.output {
            let audio_file = format!("{}_audio.ts", tempfile);
            let subtitle_file = format!("{}_subtitles.srt", tempfile);
            let mut args = vec!["-i", &tempfile];

            if self.audio_stream {
                args.push("-i");
                args.push(&audio_file);
            }

            if self.subtitle_stream {
                args.push("-i");
                args.push(&subtitle_file);
            }

            if std::path::Path::new(output).exists() {
                std::fs::remove_file(output)?;
            }

            args.push("-c");
            args.push("copy");
            args.push(output);

            println!("Executing `ffmpeg {}`", args.join(" "));
            std::process::Command::new("ffmpeg")
                .args(args)
                .stderr(std::process::Stdio::null())
                .spawn()?
                .wait()?;
            
            std::fs::remove_file(tempfile)?;
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let mut downloader = DownloadState::new()?;

    if let Err(e) = downloader.scrape_webpage() {
        println!("{} {}", "Error:".colorize("bold red"), e);
        std::process::exit(1);
    }

    match downloader.segments() {
        Err(e) => {
            println!("{} {}", "Error:".colorize("bold red"), e);
        }
        Ok(segments) => downloader.download(&segments, downloader.determine_output())?,
    }

    Ok(())
}
