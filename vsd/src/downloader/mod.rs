mod encryption;
mod fetch;
mod fix;
mod mux;
mod parse;
mod stream;
mod subtitle;

pub use encryption::Decrypter;
pub use fetch::fetch_playlist;
pub use parse::{list_all_streams, parse_all_streams, parse_selected_streams};
pub use subtitle::download_subtitle_streams;

use crate::{
    playlist::{MediaPlaylist, MediaType},
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

pub static MAX_RETRIES: AtomicU8 = AtomicU8::new(5);
pub static MAX_THREADS: AtomicU8 = AtomicU8::new(5);
pub static RUNNING: AtomicBool = AtomicBool::new(true);
pub static SKIP_MERGE: AtomicBool = AtomicBool::new(false);

#[allow(clippy::too_many_arguments)]
pub async fn download(
    base_url: Option<Url>,
    client: Client,
    decrypter: Decrypter,
    directory: Option<PathBuf>,
    no_decrypt: bool,
    output: Option<PathBuf>,
    query: HashMap<String, String>,
    mut streams: Vec<MediaPlaylist>,
    subs_codec: String,
) -> Result<()> {
    let should_mux = mux::should_mux(no_decrypt, SKIP_MERGE.load(Ordering::SeqCst), output.as_ref(), &streams);

    if should_mux && utils::find_ffmpeg().is_none() {
        bail!("ffmpeg couldn't be found, it is required to continue further.");
    }

    if !no_decrypt {
        encryption::check_unsupported_encryptions(&streams)?;
        let default_kids =
            encryption::extract_default_kids(&base_url, &client, &streams, &query).await?;
        encryption::check_key_exists_for_kid(&decrypter, &default_kids)?;
    }

    if let Some(directory) = &directory
        && !directory.exists()
    {
        fs::create_dir_all(directory)?;
    }

    for stream in &mut streams {
        if stream.media_type != MediaType::Subtitles {
            stream.split_segment(&base_url, &client, &query).await?;
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
        &base_url,
        &client,
        directory.as_ref(),
        &streams,
        &query,
        &mut temp_files,
    )
    .await?;

    stream::download_streams(
        &base_url,
        &client,
        decrypter,
        directory.as_ref(),
        no_decrypt,
        &query,
        streams,
        &mut temp_files,
    )
    .await?;

    if should_mux {
        mux::ffmpeg(output.as_ref(), &subs_codec, &temp_files).await?;
        mux::delete_temp_files(directory.as_ref(), &temp_files).await?;
    }

    Ok(())
}
