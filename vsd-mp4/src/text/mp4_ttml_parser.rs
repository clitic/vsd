/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/a4e926772e1b754fe968ee6f97490f08a40fe535/lib/text/mp4_ttml_parser.js

*/

use super::{ttml_text_parser, Subtitles};
use crate::{parser, parser::Mp4Parser, Error, Result};
use std::sync::{Arc, Mutex};

/// Parse ttml subtitles from mp4 files.
pub struct Mp4TtmlParser;

impl Mp4TtmlParser {
    /// Parse intialization segment, a valid `stpp` box should be present.
    pub fn parse_init(data: &[u8]) -> Result<Self> {
        let saw_stpp = Arc::new(Mutex::new(false));
        let saw_stpp_c = saw_stpp.clone();

        Mp4Parser::default()
            ._box("moov", Arc::new(parser::children))
            ._box("trak", Arc::new(parser::children))
            ._box("mdia", Arc::new(parser::children))
            ._box("minf", Arc::new(parser::children))
            ._box("stbl", Arc::new(parser::children))
            .full_box("stsd", Arc::new(parser::sample_description))
            ._box(
                "stpp",
                Arc::new(move |mut _box| {
                    *saw_stpp_c.lock().unwrap() = true;
                    _box.parser.stop();
                    Ok(())
                }),
            )
            .parse(data, None, None)?;

        let saw_stpp = *saw_stpp.lock().unwrap();

        if !saw_stpp {
            return Err(Error::new("STPP box not found"));
        }

        Ok(Self)
    }

    /// Parse media segments, only if valid `mdat` box(s) are present.
    pub fn parse_media(&self, data: &[u8]) -> Result<Subtitles> {
        let saw_mdat = Arc::new(Mutex::new(false));
        let cues = Arc::new(Mutex::new(vec![]));

        let saw_mdat_c = saw_mdat.clone();
        let cues_c = cues.clone();

        Mp4Parser::default()
            ._box(
                "mdat",
                parser::alldata(Arc::new(move |data| {
                    *saw_mdat_c.lock().unwrap() = true;
                    // Join this to any previous payload, in case the mp4 has multiple
                    // mdats.
                    let xml = String::from_utf8(data).map_err(|_| {
                        Error::new_decode_err("MDAT box payload as valid utf-8 data")
                    })?;
                    cues_c.lock().unwrap().append(
                        &mut ttml_text_parser::parse(&xml)
                            .map_err(|x| {
                                Error::new_decode_err(format!(
                                    "xml string as ttml content.\n\n{}\n\n{:#?}",
                                    xml, x
                                ))
                            })?
                            .into_cues(),
                    );
                    Ok(())
                })),
            )
            .parse(data, Some(false), None)?;

        let saw_mdat = *saw_mdat.lock().unwrap();

        if !saw_mdat {
            return Err(Error::new("MDAT box not found"));
        }

        let cues = cues.lock().unwrap().clone();
        Ok(Subtitles::new(cues))
    }
}
