/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/7098f43f70119226bca2e5583833aaf27b498e33/lib/media/segment_utils.js#L547-L573
    2. https://github.com/shaka-project/shaka-player/blob/7098f43f70119226bca2e5583833aaf27b498e33/lib/util/mp4_box_parsers.js#L554-L567

*/

use crate::{
    Result, parser,
    parser::{Mp4Parser, ParsedBox},
};
use std::{cell::RefCell, rc::Rc};

/// Parse default kid from mp4 `TENC` box.
pub fn default_kid(data: &[u8]) -> Result<Option<String>> {
    let default_kid = Rc::new(RefCell::new(None));
    let default_kid_c = default_kid.clone();

    Mp4Parser::new()
        .base_box("moov", parser::children)
        .base_box("trak", parser::children)
        .base_box("mdia", parser::children)
        .base_box("minf", parser::children)
        .base_box("stbl", parser::children)
        .full_box("stsd", parser::sample_description)
        .base_box("encv", parser::visual_sample_entry)
        .base_box("enca", parser::audio_sample_entry)
        .base_box("sinf", parser::children)
        .base_box("schi", parser::children)
        .full_box("tenc", move |mut _box| {
            *default_kid_c.borrow_mut() = Some(parse_tenc(&mut _box)?);
            Ok(())
        })
        .parse(data, true, false)?;

    let default_kid = default_kid.borrow();
    Ok(default_kid.clone())
}

fn parse_tenc(box_: &mut ParsedBox) -> Result<String> {
    let reader = &mut box_.reader;

    // reader.read_u8()?; // TENC box reserved
    // reader.read_u8()?; // TENC box
    // reader.read_u8()?; // TENC box is protected
    // reader.read_u8()?; // TENC box per sample iv size

    reader.skip(4)?;

    let default_kid = reader.read_bytes_u8(16)?;
    let default_kid = hex::encode(default_kid);

    Ok(default_kid)
}
