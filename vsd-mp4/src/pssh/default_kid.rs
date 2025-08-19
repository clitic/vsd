/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/7098f43f70119226bca2e5583833aaf27b498e33/lib/media/segment_utils.js#L547-L573
    2. https://github.com/shaka-project/shaka-player/blob/7098f43f70119226bca2e5583833aaf27b498e33/lib/util/mp4_box_parsers.js#L554-L567

*/

use crate::{
    Error, Result, parser,
    parser::{Mp4Parser, ParsedBox},
};
use std::sync::{Arc, Mutex};

/// Parse default kid from mp4 `TENC` box.
pub fn default_kid(data: &[u8]) -> Result<Option<String>> {
    let default_kid = Arc::new(Mutex::new(None));
    let default_kid_c = default_kid.clone();

    Mp4Parser::default()
        .basic_box("moov", Arc::new(parser::children))
        .basic_box("trak", Arc::new(parser::children))
        .basic_box("mdia", Arc::new(parser::children))
        .basic_box("minf", Arc::new(parser::children))
        .basic_box("stbl", Arc::new(parser::children))
        .full_box("stsd", Arc::new(parser::sample_description))
        .basic_box("encv", Arc::new(parser::visual_sample_entry))
        .basic_box("enca", Arc::new(parser::audio_sample_entry))
        .basic_box("sinf", Arc::new(parser::children))
        .basic_box("schi", Arc::new(parser::children))
        .full_box(
            "tenc",
            Arc::new(move |mut _box| {
                *default_kid_c.lock().unwrap() = Some(parse_tenc(&mut _box)?);
                Ok(())
            }),
        )
        .parse(data, true, false)?;

    let default_kid = default_kid.lock().unwrap();
    Ok(default_kid.clone())
}

fn parse_tenc(_box: &mut ParsedBox) -> Result<String> {
    let reader = &mut _box.reader;

    // reader
    //     .read_u8()
    //     .map_err(|_| Error::new_read("TENC box reserved (u8)."))?;
    // reader
    //     .read_u8()
    //     .map_err(|_| Error::new_read("TENC box (u8)."))?;
    // reader
    //     .read_u8()
    //     .map_err(|_| Error::new_read("TENC box is protected (u8)."))?;
    // reader
    //     .read_u8()
    //     .map_err(|_| Error::new_read("TENC box per sample iv size (u8)."))?;

    reader
        .skip(4)
        .map_err(|_| Error::new_read("TENC box (4 bytes)."))?;

    let default_kid = reader
        .read_bytes_u8(16)
        .map_err(|_| Error::new_read("TENC box default kid (16 bytes)."))?;
    let default_kid = hex::encode(default_kid);

    Ok(default_kid)
}
