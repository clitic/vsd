/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/a4e926772e1b754fe968ee6f97490f08a40fe535/lib/text/mp4_vtt_parser.js

*/

use crate::{
    Reader, Result, bail, err, parser,
    parser::Mp4Parser,
    text::{
        Cue, Subtitles,
        boxes::{MDHDBox, TFDTBox, TFHDBox, TRUNBox, TRUNSample},
    },
};
use std::{cell::RefCell, rc::Rc};

/// Parse vtt subtitles from mp4 files.
pub struct Mp4VttParser {
    /// The current time scale used by the VTT parser.
    pub timescale: u32,
}

impl Mp4VttParser {
    /// Parse intialization segment, a valid `wvtt` box should be present.
    pub fn parse_init(data: &[u8]) -> Result<Self> {
        let saw_wvtt = Rc::new(RefCell::new(false));
        let timescale = Rc::new(RefCell::new(None));

        let saw_wvtt_c = saw_wvtt.clone();
        let timescale_c = timescale.clone();

        Mp4Parser::new()
            .base_box("moov", parser::children)
            .base_box("trak", parser::children)
            .base_box("mdia", parser::children)
            .full_box("mdhd", move |mut _box| {
                let _box_version = _box.version.unwrap();
                if _box_version != 0 && _box_version != 1 {
                    bail!("MDHD box version can only be 0 or 1.");
                }
                let parsed_mdhd_box = MDHDBox::parse(&mut _box.reader, _box_version)?;
                *timescale_c.borrow_mut() = Some(parsed_mdhd_box.timescale);
                Ok(())
            })
            .base_box("minf", parser::children)
            .base_box("stbl", parser::children)
            .full_box("stsd", parser::sample_description)
            .base_box("wvtt", move |_box| {
                // A valid vtt init segment, though we have no actual subtitles yet.
                *saw_wvtt_c.borrow_mut() = true;
                Ok(())
            })
            .parse(data, false, false)?;

        let saw_wvtt = *saw_wvtt.borrow();
        let timescale = *timescale.borrow();

        if !saw_wvtt {
            bail!("WVTT box not found.");
        }

        if let Some(timescale) = timescale {
            Ok(Self { timescale })
        } else {
            Err(err!("missing timescale (should exist inside MDHD box)."))
        }
    }

    /// Parse media segments, only if valid `mdat` box(s) are present.
    pub fn parse_media(&self, data: &[u8], period_start: Option<f32>) -> Result<Subtitles> {
        let period_start = period_start.unwrap_or(0.0);

        let base_time = Rc::new(RefCell::new(0_u64));
        let presentations = Rc::new(RefCell::new(vec![]));
        let saw_tfdt = Rc::new(RefCell::new(false));
        let saw_trun = Rc::new(RefCell::new(false));
        let default_duration = Rc::new(RefCell::new(None));
        let cues = Rc::new(RefCell::new(vec![]));

        let base_time_c = base_time.clone();
        let presentations_c = presentations.clone();
        let saw_tfdt_c = saw_tfdt.clone();
        let saw_trun_c = saw_trun.clone();
        let default_duration_c = default_duration.clone();
        let cues_c = cues.clone();

        let timescale = self.timescale;

        Mp4Parser::new()
            .base_box("moof", parser::children)
            .base_box("traf", parser::children)
            .full_box("tfdt", move |mut _box| {
                *saw_tfdt_c.borrow_mut() = true;

                let _box_version = _box.version.unwrap();
                if _box_version != 0 && _box_version != 1 {
                    bail!("TFDT version can only be 0 or 1.");
                }

                let parsed_tfdt_box = TFDTBox::parse(&mut _box.reader, _box_version)?;
                *base_time_c.borrow_mut() = parsed_tfdt_box.base_media_decode_time;
                Ok(())
            })
            .full_box("tfhd", move |mut box_| {
                if box_.flags.is_none() {
                    bail!("TFHD box should have a valid flags value.");
                }

                let parsed_tfhd_box = TFHDBox::parse(&mut box_.reader, box_.flags.unwrap())?;
                *default_duration_c.borrow_mut() = parsed_tfhd_box.default_sample_duration;
                Ok(())
            })
            .full_box("trun", move |mut box_| {
                *saw_trun_c.borrow_mut() = true;
                if box_.version.is_none() {
                    bail!("TRUN box should have a valid version value.");
                }
                if box_.flags.is_none() {
                    bail!("TRUN box should have a valid flags value.");
                }

                let parsed_trun_box =
                    TRUNBox::parse(&mut box_.reader, box_.version.unwrap(), box_.flags.unwrap())?;
                *presentations_c.borrow_mut() = parsed_trun_box.sample_data;
                Ok(())
            })
            .base_box(
                "mdat",
                parser::alldata(move |data| {
                    let base_time = *base_time.borrow();
                    let presentations = presentations.borrow();
                    let saw_tfdt = *saw_tfdt.borrow();
                    let saw_trun = *saw_trun.borrow();
                    let default_duration = *default_duration.borrow();

                    if !saw_tfdt && !saw_trun {
                        bail!("Some required boxes (either TFDT or TRUN) are missing.");
                    }

                    let parsed_cues = parse_mdat(
                        timescale,
                        period_start,
                        base_time,
                        default_duration,
                        &presentations,
                        &data,
                    )?;
                    cues_c.borrow_mut().extend(parsed_cues);

                    Ok(())
                }),
            )
            .parse(data, false, false)?;

        let cues = cues.borrow().clone();
        Ok(Subtitles::new(cues))
    }
}

fn parse_mdat(
    timescale: u32,
    period_start: f32,
    base_time: u64,
    default_duration: Option<u32>,
    presentations: &[TRUNSample],
    raw_payload: &[u8],
) -> Result<impl IntoIterator<Item = Cue>> {
    let mut cues = vec![];
    let mut current_time = base_time;
    let mut reader = Reader::new_big_endian(raw_payload);

    for presentation in presentations {
        // If one presentation corresponds to multiple payloads, it is assumed
        // that all of those payloads have the same start time and duration.
        let duration = presentation.sample_duration.or(default_duration);
        let start_time = if let Some(sample_composition_time_offset) =
            presentation.sample_composition_time_offset
        {
            base_time + sample_composition_time_offset as u64
        } else {
            current_time
        };

        current_time = start_time + duration.unwrap_or(0) as u64;

        // Read samples until it adds up to the given size.
        let mut total_size = 0;
        loop {
            // Read the payload size.
            let payload_size = reader.read_u32()? as i32;
            total_size += payload_size;

            // Skip the type.
            let payload_type = reader.read_u32()?;
            let payload_name = parser::type_to_string(payload_type as usize)?;

            // Read the data payload.
            let mut payload = None;
            if payload_name == "vttc" {
                if payload_size > 8 {
                    payload = Some(reader.read_bytes_u8((payload_size - 8) as usize)?);
                }
            } else if payload_name == "vtte" {
                // It's a vtte, which is a vtt cue that is empty. Ignore any data that
                // does exist.
                reader.skip((payload_size - 8) as u64)?;
            } else {
                // println!("Unknown box {} ! Skipping!", payload_name);
                reader.skip((payload_size - 8) as u64)?;
            }

            if duration.is_some() {
                if let Some(payload) = payload {
                    // goog.asserts.assert(
                    //     this.timescale_ != null, 'Timescale should not be null!');

                    let cue = parse_vttc(
                        &payload,
                        period_start + start_time as f32 / timescale as f32,
                        period_start + current_time as f32 / timescale as f32,
                    )?;
                    cues.push(cue);
                }
            } else {
                bail!("WVTT sample duration unknown, and no default found.");
            }

            if !(presentation.sample_size.is_none()
                || total_size <= presentation.sample_size.unwrap_or(0) as i32)
            {
                bail!("The samples do not fit evenly into the sample sizes given in the TRUN box.");
            };

            // If no sampleSize was specified, it's assumed that this presentation
            // corresponds to only a single cue.
            if !(presentation.sample_size.is_some()
                && (total_size < presentation.sample_size.unwrap_or(0) as i32))
            {
                break;
            }
        }
    }

    if reader.has_more_data() {
        bail!("MDAT which contain VTT cues and non-VTT data are not currently supported.");
    };

    Ok(cues.into_iter().flatten())
}

/// Parses a vttc box into a cue.
fn parse_vttc(data: &[u8], start_time: f32, end_time: f32) -> Result<Option<Cue>> {
    let payload = Rc::new(RefCell::new(String::new()));
    let id = Rc::new(RefCell::new(String::new()));
    let settings = Rc::new(RefCell::new(String::new()));

    let payload_c = payload.clone();
    let id_c = id.clone();
    let settings_c = settings.clone();

    Mp4Parser::new()
        .base_box(
            "payl",
            parser::alldata(move |data| {
                *payload_c.borrow_mut() = String::from_utf8(data)?;
                Ok(())
            }),
        )
        .base_box(
            "iden",
            parser::alldata(move |data| {
                *id_c.borrow_mut() = String::from_utf8(data)?;
                Ok(())
            }),
        )
        .base_box(
            "sttg",
            parser::alldata(move |data| {
                *settings_c.borrow_mut() = String::from_utf8(data)?;
                Ok(())
            }),
        )
        .parse(data, false, false)?;

    let payload = payload.borrow().to_owned();

    if !payload.is_empty() {
        let id = id.borrow().to_owned();
        let settings = settings.borrow().to_owned();
        return Ok(Some(Cue {
            payload,
            _id: id,
            settings,
            start_time,
            end_time,
        }));
    }

    Ok(None)
}
