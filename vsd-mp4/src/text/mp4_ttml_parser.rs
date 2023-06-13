/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/a4e926772e1b754fe968ee6f97490f08a40fe535/lib/text/mp4_ttml_parser.js

*/

use super::{ttml_text_parser, Cue};
use crate::{parser, parser::Mp4Parser};
use std::sync::{Arc, Mutex};

/// Parse ttml subtitles from mp4 files.
pub struct Mp4TtmlParser;

impl Mp4TtmlParser {
    /// Parse intialization segment, a valid `stpp` box should be present.
    pub fn parse_init(data: &[u8]) -> Result<Self, String> {
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
            return Err("mp4parser.mp4ttmlparser: A STPP box should have been seen (a valid ttml init segment with no actual subtitles).".to_owned());
        }

        Ok(Self)
    }

    /// Parse media segments, only if valid `mdat` box(s) are present.
    pub fn parse_media(&self, data: &[u8]) -> Result<Vec<Cue>, String> {
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
                    let xml = String::from_utf8(data).map_err(|_| "mp4parser.mp4ttmlparser.boxes.MDAT: cannot decode payload as valid utf8 string.")?;
                    cues_c.lock().unwrap().append(
                        &mut ttml_text_parser::parse(&xml)
                            .map_err(|x| format!("mp4parser.ttmltextparser: couldn't parse xml string as ttml content (failed with {}).\n\n{}", x, xml))?.to_cues()
                        );
                    Ok(())
                })),
            )
            .parse(data, Some(false), None)?;

        let saw_mdat = *saw_mdat.lock().unwrap();

        if !saw_mdat {
            return Err("mp4parser.mp4ttmlparser: cannot find MDAT box in given data.".to_owned());
        }

        let cues = cues.lock().unwrap().clone();

        Ok(cues)
    }
}
