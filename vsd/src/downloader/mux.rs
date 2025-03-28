use crate::playlist::{MediaPlaylist, MediaType};
use std::{ffi::OsStr, path::PathBuf};

pub fn should_mux(streams: &[MediaPlaylist], output: Option<&PathBuf>) -> bool {
    let mut should_mux = true; // !no_decrypt && !no_merge

    // Check if output file extension matches with actual stream file extension.
    if streams.len() == 1 && output.is_some() {
        if output.unwrap().extension() == Some(OsStr::new(&streams.first().unwrap().extension())) {
            should_mux = false;
        }
    }

    if streams
        .iter()
        .filter(|x| x.media_type == MediaType::Video)
        .count()
        > 1
    {
        should_mux = false;
    }

    should_mux
}

pub struct Stream {
    pub language: Option<String>,
    pub media_type: MediaType,
    pub path: PathBuf,
}
