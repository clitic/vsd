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
use kdam::{BarExt, Column, RichProgress, tqdm};
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
    streams: Vec<MediaPlaylist>,
    retries: u8,
    threads: u8,
) -> Result<()> {
    // -----------------------------------------------------------------------------------------
    // Decide Whether To Mux Streams
    // -----------------------------------------------------------------------------------------

    let should_mux = mux::should_mux(no_decrypt, no_merge, output.as_ref(), &streams);

    if should_mux && utils::find_ffmpeg().is_none() {
        bail!("ffmpeg couldn't be found, it is required to continue further.");
    }

    // -----------------------------------------------------------------------------------------
    // Parse Key Ids From Initialization Vector
    // -----------------------------------------------------------------------------------------

    if !no_decrypt {
        encryption::check_unsupported_encryptions(&streams)?;
        let default_kids = encryption::extract_default_kids(&base_url, &client, &streams, query)?;
        encryption::check_key_exists_for_kid(&decrypter, &default_kids)?;
    }

    // -----------------------------------------------------------------------------------------
    // Prepare Download Directory & Store Streams Download Paths
    // -----------------------------------------------------------------------------------------

    let mut temp_files = vec![];

    if let Some(directory) = &directory {
        if !directory.exists() {
            fs::create_dir_all(directory)?;
        }
    }

    // -----------------------------------------------------------------------------------------
    // Prepare Progress Bar
    // -----------------------------------------------------------------------------------------

    let mut pb = RichProgress::new(
        tqdm!(unit = " SEG".to_owned(), dynamic_ncols = true),
        vec![
            Column::Text("[bold blue]?".to_owned()), // downladed bytes / estimated bytes
            Column::Animation,
            Column::Percentage(0),
            Column::Text("•".to_owned()),
            Column::CountTotal, // downloaded segments / total segments
            Column::Text("•".to_owned()),
            Column::ElapsedTime,
            Column::Text("[cyan]>".to_owned()),
            Column::RemainingTime,
            Column::Text("•".to_owned()),
            Column::Rate,
        ],
    );

    // -----------------------------------------------------------------------------------------
    // Download Subtitle Streams
    // -----------------------------------------------------------------------------------------

    download_subtitle_streams(
        &base_url,
        &client,
        directory.as_ref(),
        &streams,
        &mut pb,
        query,
        &mut temp_files,
    )?;

    let mut streams = streams
        .into_iter()
        .filter(|x| x.media_type != MediaType::Subtitles)
        .collect::<Vec<_>>();

    // -----------------------------------------------------------------------------------------
    // Estimation & Segment Splitting
    // -----------------------------------------------------------------------------------------

    for stream in &mut streams {
        stream.split_segment(&base_url, &client, query)?;
    }

    // -----------------------------------------------------------------------------------------
    // Prepare Progress Bar
    // -----------------------------------------------------------------------------------------

    pb.replace(2, Column::Percentage(2));
    pb.columns.extend_from_slice(&[
        Column::Text("•".to_owned()),
        Column::Text("[yellow]?".to_owned()), // download speed
    ]);
    pb.pb
        .reset(Some(streams.iter().map(|x| x.segments.len()).sum())); // sum up all segments

    // -----------------------------------------------------------------------------------------
    // Download Video & Audio Streams
    // -----------------------------------------------------------------------------------------

    stream::download_streams(
        &base_url,
        &client,
        decrypter,
        directory.as_ref(),
        no_decrypt,
        no_merge,
        output.as_ref(),
        pb,
        query,
        retries,
        streams,
        threads,
        &mut temp_files,
    )?;

    // -----------------------------------------------------------------------------------------
    // Mux Downloaded Streams
    // -----------------------------------------------------------------------------------------

    if should_mux {
        mux::ffmpeg(output.as_ref(), &temp_files)?;
        mux::delete_temp_files(directory.as_ref(), &temp_files)?;
    }

    Ok(())
}
