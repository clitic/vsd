mod commands;
mod cookie;
mod dash;
mod downloader;
mod hls;
mod logger;
mod options;
mod playlist;
mod progress;
mod selector;
mod utils;

#[doc(hidden)]
pub use commands::Args;
pub use reqwest;

use crate::{
    downloader::{FetchedPlaylist, MAX_RETRIES, MAX_THREADS, SKIP_DECRYPT, SKIP_MERGE},
    options::{Interaction, SelectOptions},
};
use anyhow::{Ok, Result};
use reqwest::{Client, Url};
use std::{collections::HashMap, path::PathBuf, sync::atomic::Ordering};

// pub enum Source {
//     Url(Url),
//     File(PathBuf),
// }

// pub struct Input {
//     source: Source,

// }

// impl Input {
//     pub fn new(self) -> Self {
//         match self {
//             Input::Url(url) => url,
//             Input::File(path) => Url::from_file_path(path).unwrap(),
//         }
//     }
// }

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

    pub async fn download(self) -> Result<()> {
        let query = HashMap::new();
        let playlist =
            FetchedPlaylist::new(&self.input, &self.client, self.base_url.as_ref(), &query).await?;

        let master_playlist = playlist
            .as_master_playlist(
                &self.client,
                &query,
                self.select_options,
                self.interaction_type,
                false,
            )
            .await?;

        downloader::download(
            self.base_url,
            self.client,
            self.directory,
            self.keys,
            self.output,
            query,
            master_playlist.streams,
            self.subs_codec,
        )
        .await?;

        Ok(())
    }
}
