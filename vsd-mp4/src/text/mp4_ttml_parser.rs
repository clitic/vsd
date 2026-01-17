/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/a4e926772e1b754fe968ee6f97490f08a40fe535/lib/text/mp4_ttml_parser.js

*/

use crate::{
    Error, Result, bail, parser,
    parser::Mp4Parser,
    text::{Subtitles, ttml_text_parser},
};
use std::{cell::RefCell, rc::Rc};

/// Parse ttml subtitles from mp4 files.
pub struct Mp4TtmlParser;

impl Mp4TtmlParser {
    /// Parse intialization segment, a valid `stpp` box should be present.
    pub fn parse_init(data: &[u8]) -> Result<Self> {
        let saw_stpp = Rc::new(RefCell::new(false));
        let saw_stpp_c = saw_stpp.clone();

        Mp4Parser::new()
            .base_box("moov", parser::children)
            .base_box("trak", parser::children)
            .base_box("mdia", parser::children)
            .base_box("minf", parser::children)
            .base_box("stbl", parser::children)
            .full_box("stsd", parser::sample_description)
            .base_box("stpp", move |mut _box| {
                *saw_stpp_c.borrow_mut() = true;
                _box.parser.stop();
                Ok(())
            })
            .parse(data, false, false)?;

        let saw_stpp = *saw_stpp.borrow();

        if !saw_stpp {
            bail!("STPP box not found.");
        }

        Ok(Self)
    }

    /// Parse media segments, only if valid `mdat` box(s) are present.
    pub fn parse_media(&self, data: &[u8]) -> Result<Subtitles> {
        let saw_mdat = Rc::new(RefCell::new(false));
        let cues = Rc::new(RefCell::new(vec![]));

        let saw_mdat_c = saw_mdat.clone();
        let cues_c = cues.clone();

        Mp4Parser::new()
            .base_box(
                "mdat",
                parser::alldata(move |data| {
                    *saw_mdat_c.borrow_mut() = true;
                    // Join this to any previous payload, in case the mp4 has multiple
                    // mdats.
                    let xml = String::from_utf8(data)?;
                    cues_c.borrow_mut().append(
                        &mut ttml_text_parser::parse(&xml)
                            .map_err(|x| Error::XmlDecode { error: x, xml })?
                            .into_cues(),
                    );
                    Ok(())
                }),
            )
            .parse(data, false, false)?;

        let saw_mdat = *saw_mdat.borrow();

        if !saw_mdat {
            bail!("MDAT box not found.");
        }

        let cues = cues.borrow().clone();
        Ok(Subtitles::new(cues))
    }
}
