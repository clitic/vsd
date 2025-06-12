mod encryption;
mod fetch;
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
use kdam::{Column, RichProgress, tqdm};
use reqwest::{Url, blocking::Client};
use std::{collections::HashMap, fs, path::PathBuf};

#[allow(clippy::too_many_arguments)]
pub fn download(
    base_url: Option<Url>,
    client: Client,
    decrypter: Decrypter,
    directory: Option<PathBuf>,
    no_decrypt: bool,
    no_merge: bool,
    output: Option<PathBuf>,
    query: &HashMap<String, String>,
    mut streams: Vec<MediaPlaylist>,
    retries: u8,
    threads: u8,
) -> Result<()> {
    // if streams.len() == 1 && output.extension() == Some(streams.first().unwrap().extension()) {
    //     return false;
    // }

    let should_mux = mux::should_mux(no_decrypt, no_merge, output.as_ref(), &streams);

    if should_mux && utils::find_ffmpeg().is_none() {
        bail!("ffmpeg couldn't be found, it is required to continue further.");
    }

    if !no_decrypt {
        encryption::check_unsupported_encryptions(&streams)?;
        let default_kids = encryption::extract_default_kids(&base_url, &client, &streams, query)?;
        encryption::check_key_exists_for_kid(&decrypter, &default_kids)?;
    }

    let mut temp_files = vec![];

    if let Some(directory) = &directory {
        if !directory.exists() {
            fs::create_dir_all(directory)?;
        }
    }

    for stream in &mut streams {
        if stream.media_type != MediaType::Subtitles {
            stream.split_segment(&base_url, &client, query)?;
        }
    }

    let mut pb = RichProgress::new(
        tqdm!(
            dynamic_ncols = true,
            total = streams.iter().map(|x| x.segments.len()).sum(),
            unit = " SEG"
        ),
        vec![
            Column::Text("[bold blue]?".to_owned()), // downladed bytes / estimated bytes
            Column::Animation,
            Column::Percentage(0),
            Column::Text("•".to_owned()),
            Column::CountTotal, // downloaded segments / total segments
            Column::Text("•".to_owned()),
            Column::ElapsedTime,
            Column::Text(">".to_owned()),
            Column::RemainingTime,
            Column::Text("•".to_owned()),
            Column::Rate,
        ],
    );

    download_subtitle_streams(
        &base_url,
        &client,
        directory.as_ref(),
        &streams,
        &mut pb,
        query,
        &mut temp_files,
    )?;

    stream::download_streams(
        &base_url,
        &client,
        decrypter,
        directory.as_ref(),
        no_decrypt,
        no_merge,
        pb,
        query,
        retries,
        streams,
        threads,
        &mut temp_files,
    )?;

    if should_mux {
        mux::ffmpeg(output.as_ref(), &temp_files)?;
        mux::delete_temp_files(directory.as_ref(), &temp_files)?;
    }

    Ok(())
}
