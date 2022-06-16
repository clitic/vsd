use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, Context, Result};
use kdam::term::Colorizer;

use crate::merger::BinarySequence;
use crate::parse;
use crate::progress::{DownloadProgress, StreamData};
use crate::utils::*;
pub struct DownloadState {
    args: crate::args::Args,
    downloader: crate::downloader::Downloader,
    progress: DownloadProgress,
}

impl DownloadState {
    pub fn new() -> Result<Self> {
        let args = crate::args::parse();

        if args.capture {
            println!(
                "Launching chrome in headless={} mode for 3 minutes.",
                args.headless
            );
            crate::capture::run(args.input, args.headless)?;
            std::process::exit(0);
        }

        let downloader = crate::downloader::Downloader::new(
            &args.user_agent,
            &args.header,
            &args.proxy_address,
            args.enable_cookies,
            &args.cookies,
        )
        .context("Couldn't create reqwest client.")?;

        if crate::utils::find_ffmpeg_with_path().is_none() {
            println!(
                "{} couldn't be located. Visit https://www.ffmpeg.org/download.html to install it.",
                "FFMPEG".colorize("bold red"),
            );
        }

        Ok(Self {
            args,
            downloader,
            progress: DownloadProgress::new_empty(),
        })
    }

    fn get_url(&self, uri: &str) -> Result<String> {
        if uri.starts_with("http") {
            Ok(uri.to_owned())
        } else {
            if let Some(baseurl) = &self.args.baseurl {
                Ok(reqwest::Url::parse(baseurl)?.join(&uri)?.to_string())
            } else {
                Ok(reqwest::Url::parse(&self.args.input)?
                    .join(&uri)?
                    .to_string())
            }
        }
    }

    pub fn tempfile(&self) -> String {
        let path = if let Some(output) = self.args.input.split("/").find(|x| x.ends_with(".m3u8")) {
            replace_ext(output.split("?").next().unwrap(), "ts")
        } else {
            "merged.ts".to_owned()
        };

        if std::path::Path::new(&path).exists() && !self.args.resume {
            let stemed_path = std::path::Path::new(&path)
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap();

            for i in 1..9999 {
                let core_file_copy = format!("{} ({}).ts", stemed_path, i);

                if !std::path::Path::new(&core_file_copy).exists() {
                    return core_file_copy;
                }
            }
        }
        path
    }

    fn scrape_website(&mut self) -> Result<()> {
        println!("Scraping website for HLS and Dash links.");
        let resp = self.downloader.get(&self.args.input)?;
        let links = crate::utils::find_hls_dash_links(&resp.text()?);

        match links.len() {
            0 => bail!(
                "No links found on website source. Consider using {} flag then copy captured m3u8 url and rerun command with that link with same arguments.",
                "--capture".colorize("bold green")
            ),
            1 => {
                self.args.input = links[0].clone();
                println!("Found one link {}", &links[0]);
            }
            _ => {
                let mut elinks = vec![];
                for (i, link) in links.iter().enumerate() {
                    elinks.push(format!("{:#2}) {}", i + 1, link));
                }
                let index = select(
                    "Select one link:".to_string(),
                    &elinks,
                    self.args.raw_prompts.clone(),
                )?;
                self.args.input = links[index].clone();
            }
        }

        Ok(())
    }

    fn download_alternative(&mut self, master: &m3u8_rs::MasterPlaylist) -> Result<()> {
        let stream_input = self.args.input.clone();
        let audio_tempfile = format!(
            "{}_audio.ts",
            self.progress.stream.file.trim_end_matches(".ts")
        );
        let subtitle_tempfile = format!(
            "{}_subtitles.vtt",
            self.progress.stream.file.trim_end_matches(".ts")
        );
        let subtitle_output = format!(
            "{}_subtitles.srt",
            self.progress.stream.file.trim_end_matches(".ts")
        );

        for alternative in &master.alternatives {
            self.args.input = stream_input.clone();

            match alternative.media_type {
                m3u8_rs::AlternativeMediaType::Audio => {
                    if alternative.autoselect {
                        if let Some(uri) = &alternative.uri {
                            println!("Re-targeting to download audio stream.");
                            self.args.input = self.get_url(uri)?;
                            self.progress.current("audio");
                            self.progress.audio =
                                Some(StreamData::new(&self.args.input, &audio_tempfile));

                            let content = self.downloader.get_bytes(&self.args.input)?;
                            match m3u8_rs::parse_playlist_res(&content).map_err(|_| {
                                anyhow!("Couldn't parse {} playlist.", self.args.input)
                            })? {
                                m3u8_rs::Playlist::MediaPlaylist(meadia) => {
                                    self.download(&meadia.segments, audio_tempfile.clone())?;
                                }
                                _ => (),
                            }
                        }
                    }
                }

                m3u8_rs::AlternativeMediaType::Subtitles
                | m3u8_rs::AlternativeMediaType::ClosedCaptions => {
                    if alternative.autoselect {
                        if let Some(uri) = &alternative.uri {
                            println!("Re-targeting to download subtitle stream.");
                            self.args.input = self.get_url(uri)?;
                            self.progress.current("subtitle");
                            self.progress.subtitle =
                                Some(StreamData::new(&self.args.input, &subtitle_tempfile));

                            let content = self.downloader.get_bytes(&self.args.input)?;
                            match m3u8_rs::parse_playlist_res(&content).map_err(|_| {
                                anyhow!("Couldn't parse {} playlist.", self.args.input)
                            })? {
                                m3u8_rs::Playlist::MediaPlaylist(meadia) => {
                                    self.download(&meadia.segments, subtitle_tempfile.clone())?;
                                }
                                _ => (),
                            }

                            if std::path::Path::new(&subtitle_output).exists() {
                                std::fs::remove_file(&subtitle_output)?;
                            }

                            println!(
                                "Executing `ffmpeg {}`",
                                ["-i", &subtitle_tempfile, "-c", "copy", &subtitle_output]
                                    .join(" ")
                                    .colorize("cyan")
                            );
                            std::process::Command::new("ffmpeg")
                                .args(["-i", &subtitle_tempfile, "-c", "copy", &subtitle_output])
                                .stderr(std::process::Stdio::null())
                                .spawn()?
                                .wait()?;

                            std::fs::remove_file(&subtitle_tempfile)?;

                            if let Some(subtitle) = &mut self.progress.subtitle {
                                subtitle.file = subtitle_tempfile.clone();
                            }
                        }
                    }
                }

                _ => (),
            }
        }

        self.args.input = stream_input;
        self.progress.current("stream");
        Ok(())
    }

    pub fn segments(&mut self) -> Result<Vec<m3u8_rs::MediaSegment>> {
        if find_hls_dash_links(&self.args.input).len() == 0 {
            if !std::path::Path::new(&self.args.input).exists() {
                self.scrape_website()?;
            }
        }

        let content = if self.args.input.starts_with("http") {
            self.downloader.get_bytes(&self.args.input)?
        } else {
            std::fs::read_to_string(&self.args.input)?
                .as_bytes()
                .to_vec()
        };

        match m3u8_rs::parse_playlist_res(&content)
            .map_err(|_| anyhow!("Couldn't parse {} playlist.", self.args.input))?
        {
            m3u8_rs::Playlist::MasterPlaylist(master) => {
                self.args.input = if self.args.alternative {
                    self.get_url(&parse::alternative(&master, self.args.raw_prompts)?)?
                } else {
                    self.get_url(&parse::master(
                        &master,
                        &self.args.quality,
                        self.args.raw_prompts,
                    )?)?
                };

                self.progress.current("stream");
                self.progress.stream = StreamData::new(&self.args.input, &self.tempfile());
                self.progress
                    .json_file(&replace_ext(&self.progress.stream.file, "json"));

                if !self.args.alternative && !self.args.skip {
                    self.download_alternative(&master)?;
                }

                let playlist = self.downloader.get_bytes(&self.args.input).unwrap();
                match m3u8_rs::parse_playlist_res(&playlist)
                    .map_err(|_| anyhow!("Couldn't parse {} playlist.", self.args.input))?
                {
                    m3u8_rs::Playlist::MediaPlaylist(meadia) => {
                        return Ok(meadia.segments);
                    }
                    _ => bail!("Media playlist not found."),
                }
            }
            m3u8_rs::Playlist::MediaPlaylist(meadia) => {
                self.progress.current("stream");
                self.progress.stream = StreamData::new(&self.args.input, &self.tempfile());
                self.progress
                    .json_file(&replace_ext(&self.progress.stream.file, "json"));
                return Ok(meadia.segments);
            }
        }
    }

    pub fn download(
        &self,
        segments: &Vec<m3u8_rs::MediaSegment>,
        mut tempfile: String,
    ) -> Result<()> {
        if let Some(output) = &self.args.output {
            if output.ends_with(".ts") {
                tempfile = output.clone();
            }
            println!("File will be saved at {}", tempfile);
        } else {
            println!("Temporary file will be saved at {}", tempfile);
        }

        let total = segments.len();
        let mut pb = kdam::tqdm!(total = total, unit = "ts".to_owned(), dynamic_ncols = true);

        let merger = if self.args.resume {
            let merger = BinarySequence::try_from_json(
                total,
                tempfile.clone(),
                self.progress.json_file.clone(),
            )?;

            pb.set_description(format!(
                "{} / {}",
                format_bytes(merger.stored()).2,
                format_bytes(merger.estimate()).2
            ));
            pb.set_position(merger.position());

            Arc::new(Mutex::new(merger))
        } else {
            Arc::new(Mutex::new(BinarySequence::new(
                total,
                tempfile.clone(),
                self.progress.clone(),
            )?))
        };

        pb.refresh();
        let pb = Arc::new(Mutex::new(pb));
        let client = Arc::new(self.downloader.clone());
        let pool = threadpool::ThreadPool::new(self.args.threads as usize);

        for (i, segment) in segments.iter().enumerate() {
            if self.args.resume {
                let pos = merger.lock().unwrap().position();

                if pos != 0 {
                    if pos >= i + 1 {
                        continue;
                    }
                }
            }

            if let Some(m3u8_key) = &segment.key {
                if m3u8_key.method == "SAMPLE-AES" {
                    bail!("SAMPLE-AES encrypted playlists are not supported.")
                }
            }

            let key_url = match &segment.key {
                Some(m3u8_rs::Key {
                    uri: Some(link), ..
                }) => Some(self.get_url(&link)?),
                _ => None,
            };

            let segment = segment.clone();
            let pb = pb.clone();
            let merger = merger.clone();
            let client = client.clone();
            let segment_url = self.get_url(&segment.uri)?;
            let total_retries = self.args.retry_count.clone();

            pool.execute(move || {
                let mut retries = 0;

                let mut data = loop {
                    let resp = match segment.byte_range {
                        Some(m3u8_rs::ByteRange {
                            length: start,
                            offset: Some(end),
                        }) => client.get_bytes_range(&segment_url, start, start + end - 1),
                        _ => client.get_bytes(&segment_url),
                    };

                    if resp.is_ok() {
                        break resp.unwrap();
                    } else {
                        if total_retries > retries {
                            pb.lock().unwrap().write(format!(
                                "{} to download segment at index {}",
                                "Retrying".colorize("bold yellow"),
                                i + 1
                            ));
                            retries += 1;
                            continue;
                        } else {
                            pb.lock().unwrap().write(format!(
                                "{}: Reached maximum number of retries for {}",
                                "Error".colorize("bold red"),
                                segment_url
                            ));
                            std::process::exit(1);
                        }
                    }
                };

                if let Some(eku) = key_url {
                    data = crate::decrypt::HlsDecrypt::from_key(
                        segment.key.unwrap(),
                        client.get_bytes(&eku).unwrap(),
                    )
                    .decrypt(&data);
                }

                let mut merger = merger.lock().unwrap();
                merger.write(i, &data).unwrap();
                merger.flush().unwrap();

                let mut pb = pb.lock().unwrap();

                pb.set_description(format!(
                    "{} / {}",
                    format_bytes(merger.stored()).2,
                    format_bytes(merger.estimate()).2
                ));

                pb.update(1);
            });
        }

        pool.join();
        eprintln!();
        merger.lock().unwrap().flush().unwrap();

        if merger.lock().unwrap().buffered() {
            println!("File {} downloaded successfully.", tempfile);
        } else {
            bail!("File {} is not downloaded successfully.", tempfile);
        }
        Ok(())
    }

    pub fn transmux(&mut self) -> Result<()> {
        if let Some(output) = &self.args.output {
            let mut args = vec!["-i", &self.progress.stream.file];

            if let Some(audio) = &self.progress.audio {
                args.push("-i");
                args.push(&audio.file);
            }

            if let Some(subtitle) = &self.progress.subtitle {
                args.push("-i");
                args.push(&subtitle.file);
            }

            if std::path::Path::new(output).exists() {
                std::fs::remove_file(output)?;
            }

            args.push("-c");
            args.push("copy");
            args.push(output);

            println!("Executing `ffmpeg {}`", args.join(" ").colorize("cyan"));
            std::process::Command::new("ffmpeg")
                .args(args)
                .stderr(std::process::Stdio::null())
                .spawn()?
                .wait()?;

            if let Some(audio) = &self.progress.audio {
                std::fs::remove_file(&audio.file)?;
            }

            if let Some(subtitle) = &self.progress.subtitle {
                std::fs::remove_file(&subtitle.file)?;
            }

            std::fs::remove_file(&self.progress.stream.file)?;
        }

        if std::path::Path::new(&self.progress.json_file).exists() {
            std::fs::remove_file(&self.progress.json_file)?;
        }
        Ok(())
    }
}
