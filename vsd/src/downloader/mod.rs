mod encryption;
mod fetch;
mod fix;
mod mux;
mod stream;
mod subtitle;

pub use fetch::FetchedPlaylist;
pub use subtitle::download_subtitle_streams;
use vsd_mp4::pssh::PsshBox;

use crate::{
    options::{Interaction, SelectOptions},
    playlist::MediaType,
    utils,
};
use anyhow::{Result, bail};
use log::{error, warn};
use reqwest::{Client, Url};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

pub static MAX_RETRIES: AtomicU8 = AtomicU8::new(10);
pub static MAX_THREADS: AtomicU8 = AtomicU8::new(5);
pub static RUNNING: AtomicBool = AtomicBool::new(true);
pub static SKIP_DECRYPT: AtomicBool = AtomicBool::new(false);
pub static SKIP_MERGE: AtomicBool = AtomicBool::new(false);

/// Download streams from DASH or HLS playlist.
pub struct Downloader {
    input: String,
    client: Client,
    base_url: Option<Url>,
    directory: Option<PathBuf>,
    output: Option<PathBuf>,
    subs_codec: String,
    interaction_type: Interaction,
    select_options: SelectOptions,
    query: Vec<(String, String)>,
    keys: HashMap<String, String>,
}

impl Downloader {
    pub fn new(input: impl Into<String>, client: &Client) -> Self {
        Self {
            input: input.into(),
            client: client.clone(),
            base_url: None,
            directory: None,
            output: None,
            subs_codec: "copy".to_owned(),
            interaction_type: Interaction::None,
            select_options: "v=best:s=en".parse().unwrap(),
            query: Vec::new(),
            keys: HashMap::new(),
        }
    }

    /// Base URL for resolving relative segment paths.
    ///
    /// Required for local playlist files. For remote playlists,
    /// the final redirected URL is used by default.
    pub fn base_url(mut self, base_url: impl Into<Url>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Working directory for temporary segment files.
    ///
    /// Defaults to the current directory.
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// Mux downloaded streams into a video container using ffmpeg (`.mp4`, `.mkv`, etc.).
    ///
    /// Overwrites existing files and deletes intermediate stream files after muxing.
    pub fn output(mut self, output: impl Into<PathBuf>) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Subtitle codec to use when muxing with ffmpeg.
    ///
    /// Defaults to `mov_text` for `.mp4` containers, `copy` for others.
    pub fn subs_codec(mut self, subs_codec: impl Into<String>) -> Self {
        self.subs_codec = subs_codec.into();
        self
    }

    /// Enable interactive stream selection with styled or plain text prompts.
    pub fn interactive(mut self, raw: bool) -> Self {
        if raw {
            self.interaction_type = Interaction::Raw;
        } else {
            self.interaction_type = Interaction::Modern;
        }
        self
    }

    /// Stream selection filters for automatic mode.
    pub fn select_streams(mut self, select_streams: &str) -> Self {
        self.select_options = select_streams.parse().unwrap();
        self
    }

    /// Additional query parameters for requests.
    pub fn query(mut self, query: &str) -> Self {
        if query.is_empty() {
            return self;
        }
        self.query = query
            .split('&')
            .filter_map(|x| {
                if let Some((key, value)) = x.split_once('=') {
                    Some((key.to_owned(), value.to_owned()))
                } else {
                    None
                }
            })
            .collect();
        self
    }

    /// Decryption keys in `KID:KEY;…` hex format.
    pub fn keys(mut self, keys: HashMap<String, String>) -> Self {
        self.keys = keys;
        self
    }

    /// Skip decryption and download encrypted streams as-is.
    ///
    /// Ignores `--output` when enabled.
    pub fn skip_decrypt(self, skip_decrypt: bool) -> Self {
        SKIP_DECRYPT.store(skip_decrypt, Ordering::SeqCst);
        self
    }

    /// Skip segment merging and keep individual files.
    ///
    /// Ignores `--output` when enabled.
    pub fn skip_merge(self, skip_merge: bool) -> Self {
        SKIP_MERGE.store(skip_merge, Ordering::SeqCst);
        self
    }

    /// Maximum retry attempts per segment.
    pub fn max_retries(self, max_retries: u8) -> Self {
        MAX_RETRIES.store(max_retries, Ordering::SeqCst);
        self
    }

    /// Number of concurrent download threads (1–16).
    pub fn max_threads(self, max_threads: u8) -> Self {
        MAX_THREADS.store(max_threads, Ordering::SeqCst);
        self
    }

    async fn fetch_playlist(&self) -> Result<FetchedPlaylist> {
        Ok(FetchedPlaylist::new(
            &self.input,
            &self.client,
            self.base_url.as_ref(),
            &self.query,
        )
        .await?)
    }

    pub(crate) async fn list_playlist(self) -> Result<()> {
        self.fetch_playlist().await?.list_streams()?;
        Ok(())
    }

    pub(crate) async fn parse_playlist(self) -> Result<()> {
        let pl = self
            .fetch_playlist()
            .await?
            .as_master_playlist(
                &self.client,
                &self.query,
                self.select_options,
                Interaction::None,
                true,
            )
            .await?;
        if let Some(output) = &self.output {
            serde_json::to_writer(std::fs::File::create(output)?, &pl)?;
        } else {
            serde_json::to_writer(std::io::stdout(), &pl)?;
        }
        Ok(())
    }

    pub(crate) async fn pssh_playlist(self) -> Result<HashSet<Vec<u8>>> {
        let pl = self
            .fetch_playlist()
            .await?
            .as_master_playlist(
                &self.client,
                &self.query,
                self.select_options,
                Interaction::None,
                true,
            )
            .await?;

        let mut pssh_data = HashSet::new();
        for stream in pl.streams {
            let Some(init_seg) = stream.fetch_init_seg(&self.client, &self.query).await? else {
                continue;
            };
            PsshBox::from_init(&init_seg)?
                .data
                .into_iter()
                .for_each(|x| {
                    let _ = pssh_data.insert(x.data);
                });
        }
        Ok(pssh_data)
    }

    pub async fn download(self) -> Result<()> {
        let pl = self
            .fetch_playlist()
            .await?
            .as_master_playlist(
                &self.client,
                &self.query,
                self.select_options,
                self.interaction_type,
                false,
            )
            .await?;
        let mut streams = pl.streams;

        let should_mux = mux::should_mux(self.output.as_ref(), &streams);

        if should_mux && utils::find_ffmpeg().is_none() {
            bail!("ffmpeg couldn't be found, it is required to continue further.");
        }

        if !SKIP_DECRYPT.load(Ordering::SeqCst) {
            encryption::check_unsupported_encryptions(&streams)?;
            let default_kids =
                encryption::extract_default_kids(&self.client, &streams, &self.query).await?;
            encryption::check_key_exists_for_kid(&self.keys, &default_kids)?;
        }

        if let Some(directory) = &self.directory
            && !directory.exists()
        {
            fs::create_dir_all(directory)?;
        }

        for stream in &mut streams {
            if stream.media_type != MediaType::Subtitles {
                stream
                    .fetch_split_seg(&self.base_url, &self.client, &self.query)
                    .await?;
            }
        }

        let mut temp_files = vec![];

        tokio::spawn(async {
            if tokio::signal::ctrl_c().await.is_ok() && RUNNING.load(Ordering::SeqCst) {
                warn!("Ctrl+C received, stopping gracefully.");
                RUNNING.store(false, Ordering::SeqCst);
            }

            if tokio::signal::ctrl_c().await.is_ok() {
                error!("Ctrl+C received, force exiting.");
                std::process::exit(1);
            }
        });

        download_subtitle_streams(
            &self.base_url,
            &self.client,
            self.directory.as_ref(),
            &streams,
            &self.query,
            &mut temp_files,
        )
        .await?;

        stream::download_streams(
            &self.base_url,
            &self.client,
            self.directory.as_ref(),
            &self.keys,
            &self.query,
            streams,
            &mut temp_files,
        )
        .await?;

        if should_mux {
            mux::ffmpeg(self.output.as_ref(), &self.subs_codec, &temp_files).await?;
            mux::delete_temp_files(self.directory.as_ref(), &temp_files).await?;
        }

        Ok(())
    }
}
