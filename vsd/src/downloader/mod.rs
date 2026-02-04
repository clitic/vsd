mod encryption;
mod fetch;
mod fix;
mod mux;
mod stream;
mod subtitle;

pub use fetch::FetchedPlaylist;
pub use subtitle::download_subtitle_streams;

use crate::{
    options::{Interaction, SelectOptions},
    playlist::MediaType,
    utils,
};
use anyhow::{Result, bail};
use log::{error, warn};
use reqwest::{Client, Url};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

pub static MAX_RETRIES: AtomicU8 = AtomicU8::new(10);
pub static MAX_THREADS: AtomicU8 = AtomicU8::new(5);
pub static RUNNING: AtomicBool = AtomicBool::new(true);
pub static SKIP_DECRYPT: AtomicBool = AtomicBool::new(false);
pub static SKIP_MERGE: AtomicBool = AtomicBool::new(false);

/// A downloader for DASH and HLS playlists.
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
    pub fn new(input: String, client: Client) -> Self {
        Self {
            input,
            client,
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

    pub fn base_url(mut self, base_url: impl Into<Url>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    pub fn output(mut self, output: impl Into<PathBuf>) -> Self {
        self.output = Some(output.into());
        self
    }

    pub fn subs_codec(mut self, subs_codec: impl Into<String>) -> Self {
        self.subs_codec = subs_codec.into();
        self
    }

    pub fn interactive(mut self, raw: bool) -> Self {
        if raw {
            self.interaction_type = Interaction::Raw;
        } else {
            self.interaction_type = Interaction::Modern;
        }
        self
    }

    pub fn select_streams(mut self, select_streams: &str) -> Self {
        self.select_options = select_streams.parse().unwrap();
        self
    }

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

    pub fn keys(mut self, keys: HashMap<String, String>) -> Self {
        self.keys = keys;
        self
    }

    pub fn skip_decrypt(self, skip_decrypt: bool) -> Self {
        SKIP_DECRYPT.store(skip_decrypt, Ordering::SeqCst);
        self
    }

    pub fn skip_merge(self, skip_merge: bool) -> Self {
        SKIP_MERGE.store(skip_merge, Ordering::SeqCst);
        self
    }

    pub fn max_retries(self, max_retries: u8) -> Self {
        MAX_RETRIES.store(max_retries, Ordering::SeqCst);
        self
    }

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

    pub async fn list(self) -> Result<()> {
        let playlist = self.fetch_playlist().await?;
        playlist.list_streams()?;
        Ok(())
    }

    pub async fn parse(self) -> Result<()> {
        let playlist = self.fetch_playlist().await?;
        let master_playlist = playlist
            .as_master_playlist(
                &self.client,
                &self.query,
                self.select_options,
                Interaction::None,
                true,
            )
            .await?;
        if let Some(output) = &self.output {
            serde_json::to_writer(std::fs::File::create(output)?, &master_playlist)?;
        } else {
            serde_json::to_writer(std::io::stdout(), &master_playlist)?;
        }
        Ok(())
    }

    pub async fn download(self) -> Result<()> {
        let playlist = self.fetch_playlist().await?;
        let master_playlist = playlist
            .as_master_playlist(
                &self.client,
                &self.query,
                self.select_options,
                self.interaction_type,
                false,
            )
            .await?;
        let mut streams = master_playlist.streams;

        let should_mux = mux::should_mux(self.output.as_ref(), &streams);

        if should_mux && utils::find_ffmpeg().is_none() {
            bail!("ffmpeg couldn't be found, it is required to continue further.");
        }

        if !SKIP_DECRYPT.load(Ordering::SeqCst) {
            encryption::check_unsupported_encryptions(&streams)?;
            let default_kids = encryption::extract_default_kids(
                &self.base_url,
                &self.client,
                &streams,
                &self.query,
            )
            .await?;
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
