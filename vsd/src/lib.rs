mod automation;
mod commands;
mod cookie;
mod dash;
mod downloader;
mod hls;
mod logger;
mod playlist;
mod progress;
mod utils;

#[doc(hidden)]
pub use commands::Args;
pub use reqwest;

use crate::{
    automation::{InteractionType, SelectOptions},
    downloader::{MAX_RETRIES, MAX_THREADS, SKIP_DECRYPT, SKIP_MERGE},
};
use anyhow::{Ok, Result};
use reqwest::{Client, Url};
use std::{collections::HashMap, path::PathBuf, sync::atomic::Ordering};

/// A downloader for DASH and HLS playlists.
pub struct Downloader {
    client: Client,
    input: String,
    base_url: Option<Url>,
    directory: Option<PathBuf>,
    output: Option<PathBuf>,
    subs_codec: String,
    select_options: SelectOptions,
    keys: HashMap<String, String>,
}

impl Downloader {
    pub fn new(client: Client, input: String) -> Self {
        Self {
            client,
            input,
            base_url: None,
            directory: None,
            output: None,
            subs_codec: "copy".to_owned(),
            select_options: SelectOptions::parse("v=best:s=en"),
            keys: HashMap::new(),
        }
    }

    /// Base url to be used for building absolute url to segment.
    /// This flag is usually needed for local input files.
    /// By default redirected playlist url is used.
    pub fn base_url(mut self, base_url: impl Into<Url>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Change directory path for temporarily downloaded files.
    /// By default current working directory is used.
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// Mux all downloaded streams to a video container (.mp4, .mkv, etc.) using ffmpeg.
    /// Note that existing files will be overwritten and downloaded streams will be deleted.
    pub fn output(mut self, output: impl Into<PathBuf>) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Force some specific subtitle codec when muxing through ffmpeg.
    /// By default `mov_text` is used for .mp4 and `copy` for others.
    pub fn subs_codec(mut self, subs_codec: impl Into<String>) -> Self {
        self.subs_codec = subs_codec.into();
        self
    }

    /// Prompt for custom streams selection with modern style input prompts. By default proceed with defaults.
    /// Prompt for custom streams selection with raw style input prompts. By default proceed with defaults.
    pub fn interactive(self, raw: bool) -> Self {
        automation::set_interaction_type(if raw {
            InteractionType::Raw
        } else {
            InteractionType::Modern
        });
        self
    }

    /// Filters to be applied for automatic stream selection.
    pub fn select_streams(mut self, select_streams: &str) -> Self {
        self.select_options = SelectOptions::parse(select_streams);
        self
    }

    /// Keys for decrypting encrypted streams.
    /// KID:KEY should be specified in hex format.
    pub fn keys(mut self, keys: HashMap<String, String>) -> Self {
        self.keys = keys;
        self
    }

    /// Download encrypted streams without decrypting them.
    /// Note that --output flag is ignored if this flag is used.
    pub fn skip_decrypt(self, skip_decrypt: bool) -> Self {
        SKIP_DECRYPT.store(skip_decrypt, Ordering::SeqCst);
        self
    }

    /// Download streams without merging them.
    /// Note that --output flag is ignored if this flag is used.
    pub fn skip_merge(self, skip_merge: bool) -> Self {
        SKIP_MERGE.store(skip_merge, Ordering::SeqCst);
        self
    }

    /// Maximum number of retries to download an individual segment.
    pub fn max_retries(self, max_retries: u8) -> Self {
        MAX_RETRIES.store(max_retries, Ordering::SeqCst);
        self
    }

    /// Maximum number of retries to download an individual segment.
    pub fn max_threads(self, max_threads: u8) -> Self {
        MAX_THREADS.store(max_threads, Ordering::SeqCst);
        self
    }

    pub async fn download(self) -> Result<()> {
        let query = HashMap::new();
        let meta =
            downloader::fetch_playlist(self.base_url.clone(), &self.client, &self.input, &query)
                .await?;

        let streams = downloader::parse_selected_streams(
            self.base_url.clone(),
            &self.client,
            &meta,
            &query,
            self.select_options,
        )
        .await?;

        downloader::download(
            self.base_url,
            self.client,
            self.directory,
            self.keys,
            self.output,
            query,
            streams,
            self.subs_codec,
        )
        .await?;

        Ok(())
    }
}
