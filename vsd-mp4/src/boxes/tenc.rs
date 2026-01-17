/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/7098f43f70119226bca2e5583833aaf27b498e33/lib/media/segment_utils.js#L547-L573
    2. https://github.com/shaka-project/shaka-player/blob/7098f43f70119226bca2e5583833aaf27b498e33/lib/util/mp4_box_parsers.js#L554-L567

*/

use crate::{
    ParsedBox, Result,
    parser::{self, Mp4Parser},
};
use std::{cell::RefCell, rc::Rc};

/// Parse default kid from mp4 `TENC` box.
pub struct TencBox {
    pub default_kid: String,
}

impl TencBox {
    pub fn from_init(data: &[u8]) -> Result<Option<Self>> {
        let tenc_box = Rc::new(RefCell::new(None));
        let tenc_box_c = tenc_box.clone();

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
            .full_box("tenc", move |mut box_| {
                *tenc_box_c.borrow_mut() = Some(Self::new(&mut box_)?);
                Ok(())
            })
            .parse(data, true, false)?;

        Ok(tenc_box.take())
    }

    pub fn new(box_: &mut ParsedBox) -> Result<TencBox> {
        let reader = &mut box_.reader;

        // reader.read_u8()?; // TENC box reserved
        // reader.read_u8()?; // TENC box
        // reader.read_u8()?; // TENC box is protected
        // reader.read_u8()?; // TENC box per sample iv size

        reader.skip(4)?;

        let default_kid = reader.read_bytes_u8(16)?;
        let default_kid = hex::encode(default_kid);

        Ok(Self { default_kid })
    }
}
