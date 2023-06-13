/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/a4e926772e1b754fe968ee6f97490f08a40fe535/lib/text/mp4_vtt_parser.js

*/

use super::Cue;
use crate::{
    boxes::{MDHDBox, TFDTBox, TFHDBox, TRUNBox, TRUNSample},
    parser,
    parser::Mp4Parser,
    Reader,
};
use std::sync::{Arc, Mutex};

/// Parse vtt subtitles from mp4 files.
pub struct Mp4VttParser {
    /// The current time scale used by the VTT parser.
    pub timescale: u32,
}

impl Mp4VttParser {
    /// Parse intialization segment, a valid `wvtt` box should be present.
    pub fn parse_init(data: &[u8]) -> Result<Self, String> {
        let saw_wvtt = Arc::new(Mutex::new(false));
        let timescale = Arc::new(Mutex::new(None));

        let saw_wvtt_c = saw_wvtt.clone();
        let timescale_c = timescale.clone();

        Mp4Parser::default()
            ._box("moov", Arc::new(parser::children))
            ._box("trak", Arc::new(parser::children))
            ._box("mdia", Arc::new(parser::children))
            .full_box(
                "mdhd",
                Arc::new(move |mut _box| {
                    let _box_version = _box.version.unwrap();
                    if _box_version != 0 && _box_version != 1 {
                        return Err(
                            "mp4parser.mp4vttparser: MDHD version can only be 0 or 1.".to_owned()
                        );
                    }
                    let parsed_mdhd_box =
                        MDHDBox::parse(&mut _box.reader, _box_version).map_err(|x| {
                            x.replace("mp4parser.boxes.MDHD", "mp4parser.mp4vttparser.boxes.MDHD")
                        })?;
                    *timescale_c.lock().unwrap() = Some(parsed_mdhd_box.timescale);
                    Ok(())
                }),
            )
            ._box("minf", Arc::new(parser::children))
            ._box("stbl", Arc::new(parser::children))
            .full_box("stsd", Arc::new(parser::sample_description))
            ._box(
                "wvtt",
                Arc::new(move |_box| {
                    // A valid vtt init segment, though we have no actual subtitles yet.
                    *saw_wvtt_c.lock().unwrap() = true;
                    Ok(())
                }),
            )
            .parse(data, None, None)?;

        let saw_wvtt = *saw_wvtt.lock().unwrap();
        let timescale = *timescale.lock().unwrap();

        if !saw_wvtt {
            return Err("mp4parser.mp4vttparser: A WVTT box should have been seen (a valid vtt init segment with no actual subtitles).".to_owned());
        }

        if let Some(timescale) = timescale {
            Ok(Self { timescale })
        } else {
            Err("mp4parser.mp4vttparser: Missing timescale for VTT content. It should be located in the MDHD.".to_owned())
        }
    }

    /// Parse media segments, only if valid `mdat` box(s) are present.
    pub fn parse_media(&self, data: &[u8], period_start: Option<f32>) -> Result<Vec<Cue>, String> {
        let period_start = period_start.unwrap_or(0.0);

        let base_time = Arc::new(Mutex::new(0_u64));
        let presentations = Arc::new(Mutex::new(vec![]));
        let saw_tfdt = Arc::new(Mutex::new(false));
        let saw_trun = Arc::new(Mutex::new(false));
        let default_duration = Arc::new(Mutex::new(None));
        let cues = Arc::new(Mutex::new(vec![]));

        let base_time_c = base_time.clone();
        let presentations_c = presentations.clone();
        let saw_tfdt_c = saw_tfdt.clone();
        let saw_trun_c = saw_trun.clone();
        let default_duration_c = default_duration.clone();
        let cues_c = cues.clone();

        let timescale = self.timescale;

        Mp4Parser::default()
            ._box("moof", Arc::new(parser::children))
            ._box("traf", Arc::new(parser::children))
            .full_box(
                "tfdt",
                Arc::new(move |mut _box| {
                    *saw_tfdt_c.lock().unwrap() = true;

                    let _box_version = _box.version.unwrap();
                    if _box_version != 0 && _box_version != 1 {
                        return Err(
                            "mp4parser.mp4vttparser: TFDT version can only be 0 or 1".to_owned()
                        );
                    }

                    let parsed_tfdt_box =
                        TFDTBox::parse(&mut _box.reader, _box_version).map_err(|x| {
                            x.replace("mp4parser.boxes.TFDT", "mp4parser.mp4vttparser.boxes.TFDT")
                        })?;
                    *base_time_c.lock().unwrap() = parsed_tfdt_box.base_media_decode_time;
                    Ok(())
                }),
            )
            .full_box(
                "tfhd",
                Arc::new(move |mut _box| {
                    if _box.flags.is_none() {
                        return Err(
                            "mp4parser.mp4vttparser: A TFHD box should have a valid flags value."
                                .to_owned(),
                        );
                    }

                    let parsed_tfhd_box = TFHDBox::parse(&mut _box.reader, _box.flags.unwrap())
                        .map_err(|x| {
                            x.replace("mp4parser.boxes.TFHD", "mp4parser.mp4vttparser.boxes.TFHD")
                        })?;
                    *default_duration_c.lock().unwrap() = parsed_tfhd_box.default_sample_duration;
                    Ok(())
                }),
            )
            .full_box(
                "trun",
                Arc::new(move |mut _box| {
                    *saw_trun_c.lock().unwrap() = true;
                    if _box.version.is_none() {
                        return Err(
                            "mp4parser.mp4vttparser: A TRUN box should have a valid version value."
                                .to_owned(),
                        );
                    }
                    if _box.flags.is_none() {
                        return Err(
                            "mp4parser.mp4vttparser: A TRUN box should have a valid flags value."
                                .to_owned(),
                        );
                    }

                    let parsed_trun_box = TRUNBox::parse(
                        &mut _box.reader,
                        _box.version.unwrap(),
                        _box.flags.unwrap(),
                    )
                    .map_err(|x| {
                        x.replace("mp4parser.boxes.TRUN", "mp4parser.mp4vttparser.boxes.TRUN")
                    })?;
                    *presentations_c.lock().unwrap() = parsed_trun_box.sample_data;
                    Ok(())
                }),
            )
            ._box(
                "mdat",
                parser::alldata(Arc::new(move |data| {
                    let base_time = *base_time.lock().unwrap();
                    let presentations = presentations.lock().unwrap();
                    let saw_tfdt = *saw_tfdt.lock().unwrap();
                    let saw_trun = *saw_trun.lock().unwrap();
                    let default_duration = *default_duration.lock().unwrap();

                    if !saw_tfdt && !saw_trun {
                        return Err("mp4parser.mp4vttparser: A required box is missing.".to_owned());
                    }

                    let parsed_cues = parse_mdat(
                        timescale,
                        period_start,
                        base_time,
                        default_duration,
                        &presentations,
                        &data,
                    )?;
                    cues_c.lock().unwrap().extend(parsed_cues);

                    Ok(())
                })),
            )
            .parse(data, Some(false), None)?;

        let cues = cues.lock().unwrap().clone();
        Ok(cues)
    }
}

fn parse_mdat(
    timescale: u32,
    period_start: f32,
    base_time: u64,
    default_duration: Option<u32>,
    presentations: &[TRUNSample],
    raw_payload: &[u8],
) -> Result<impl IntoIterator<Item = Cue>, String> {
    let mut cues = vec![];
    let mut current_time = base_time;
    let mut reader = Reader::new(raw_payload, false);

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
            let payload_size = reader
                .read_u32()
                .map_err(|_| "mp4parser.mp4vttparser: cannot read payload size (u32).".to_owned())?
                as i32;
            total_size += payload_size;

            // Skip the type.
            let payload_type = reader.read_u32().map_err(|_| {
                "mp4parser.mp4vttparser: cannot read payload type (u32).".to_owned()
            })?;
            let payload_name = parser::type_to_string(payload_type as usize).map_err(|_| {
                "mp4parser.mp4vttparser: cannot decode payload name as valid utf8 string."
                    .to_owned()
            })?;

            // Read the data payload.
            let mut payload = None;
            if payload_name == "vttc" {
                if payload_size > 8 {
                    payload = Some(reader.read_bytes_u8((payload_size - 8) as usize).map_err(
                        |_| {
                            format!(
                                "mp4parser.mp4vttparser: cannot read payload data ({} bytes).",
                                payload_size - 8
                            )
                        },
                    )?);
                }
            } else if payload_name == "vtte" {
                // It's a vtte, which is a vtt cue that is empty. Ignore any data that
                // does exist.
                reader.skip((payload_size - 8) as u64).map_err(|_| {
                    format!(
                        "mp4parser.mp4vttparser: cannot read payload data ({} bytes).",
                        payload_size - 8
                    )
                })?;
            } else {
                // println!("Unknown box {} ! Skipping!", payload_name);
                reader.skip((payload_size - 8) as u64).map_err(|_| {
                    format!(
                        "mp4parser.mp4vttparser: cannot read payload data ({} bytes).",
                        payload_size - 8
                    )
                })?;
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
                return Err(
                    "mp4parser.mp4vttparser: WVTT sample duration unknown, and no default found!"
                        .to_owned(),
                );
            }

            if !(presentation.sample_size.is_none()
                || total_size <= presentation.sample_size.unwrap_or(0) as i32)
            {
                return Err(
                "mp4parser.mp4vttparser: The samples do not fit evenly into the sample sizes given in the TRUN box!".to_owned());
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
        return Err("mp4parser.mp4vttparser: MDAT which contain VTT cues and non-VTT data are not currently supported!".to_owned());
    };

    Ok(cues.into_iter().flatten())
}

/// Parses a vttc box into a cue.
fn parse_vttc(data: &[u8], start_time: f32, end_time: f32) -> Result<Option<Cue>, String> {
    let payload = Arc::new(Mutex::new(String::new()));
    let id = Arc::new(Mutex::new(String::new()));
    let settings = Arc::new(Mutex::new(String::new()));

    let payload_c = payload.clone();
    let id_c = id.clone();
    let settings_c = settings.clone();

    Mp4Parser::default()
        ._box(
            "payl",
            parser::alldata(Arc::new(move |data| {
                *payload_c.lock().unwrap() = String::from_utf8(data).map_err(|_| "mp4parser.mp4vttparser.boxes.VTTC: cannot decode payload as valid utf8 string.".to_owned())?;
                Ok(())
            })),
        )
        ._box(
            "iden",
            parser::alldata(Arc::new(move |data| {
                *id_c.lock().unwrap() = String::from_utf8(data).map_err(|_| "mp4parser.mp4vttparser.boxes.VTTC: cannot decode id as valid utf8 string.".to_owned())?;
                Ok(())
            })),
        )
        ._box(
            "sttg",
            parser::alldata(Arc::new(move |data| {
                *settings_c.lock().unwrap() = String::from_utf8(data).map_err(|_| "mp4parser.mp4vttparser.boxes.VTTC: cannot decode setting as valid utf8 string.".to_owned())?;
                Ok(())
            })),
        )
        .parse(data, None, None)?;

    let payload = payload.lock().unwrap().to_owned();

    if !payload.is_empty() {
        let id = id.lock().unwrap().to_owned();
        let settings = settings.lock().unwrap().to_owned();
        return Ok(Some(Cue {
            payload,
            id,
            settings,
            start_time,
            end_time,
        }));
    }

    Ok(None)
}
