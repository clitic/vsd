/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/a4e926772e1b754fe968ee6f97490f08a40fe535/lib/text/mp4_vtt_parser.js

*/

use crate::{
    Error, Reader, Result, bail,
    boxes::{MdhdBox, TfdtBox, TfhdBox, TrunBox, TrunSample},
    parser::{self, Mp4Parser},
    text::{Cue, Subtitles},
};
use std::{cell::RefCell, rc::Rc};

/// Parse vtt subtitles from mp4 files.
pub struct Mp4VttParser {
    /// The current time scale used by the VTT parser.
    pub timescale: u32,
}

impl Mp4VttParser {
    /// Parse intialization segment, a valid `wvtt` box should be present.
    pub fn from_init(data: &[u8]) -> Result<Self> {
        let saw_wvtt = Rc::new(RefCell::new(false));
        let timescale = Rc::new(RefCell::new(None));

        let saw_wvtt_c = saw_wvtt.clone();
        let timescale_c = timescale.clone();

        Mp4Parser::new()
            .base_box("moov", parser::children)
            .base_box("trak", parser::children)
            .base_box("mdia", parser::children)
            .full_box("mdhd", move |mut box_| {
                let box_version = box_.version.unwrap();

                if box_version != 0 && box_version != 1 {
                    bail!("MDHD box version can only be 0 or 1.");
                }

                let mdhd_box = MdhdBox::new(&mut box_)?;
                *timescale_c.borrow_mut() = Some(mdhd_box.timescale);
                Ok(())
            })
            .base_box("minf", parser::children)
            .base_box("stbl", parser::children)
            .full_box("stsd", parser::sample_description)
            .base_box("wvtt", move |_| {
                // A valid vtt init segment, though we have no actual subtitles yet.
                *saw_wvtt_c.borrow_mut() = true;
                Ok(())
            })
            .parse(data, false, false)?;

        if !saw_wvtt.take() {
            bail!("WVTT box not found.");
        }

        if let Some(timescale) = timescale.take() {
            Ok(Self { timescale })
        } else {
            Err(Error::Generic(
                "Missing timescale (should exist inside MDHD box).".to_owned(),
            ))
        }
    }

    /// Parse media segments, only if valid `mdat` box(s) are present.
    pub fn parse(&self, data: &[u8], period_start: Option<f32>) -> Result<Subtitles> {
        let period_start = period_start.unwrap_or_default();
        let timescale = self.timescale;

        let base_time = Rc::new(RefCell::new(0_u64));
        let default_duration = Rc::new(RefCell::new(None));
        let presentations = Rc::new(RefCell::new(Vec::new()));
        let saw_tfdt = Rc::new(RefCell::new(false));
        let saw_trun = Rc::new(RefCell::new(false));
        let subtitles = Rc::new(RefCell::new(Subtitles::new()));

        let base_time_c = base_time.clone();
        let default_duration_c = default_duration.clone();
        let presentations_c = presentations.clone();
        let saw_tfdt_c = saw_tfdt.clone();
        let saw_trun_c = saw_trun.clone();
        let subtitles_c = subtitles.clone();

        Mp4Parser::new()
            .base_box("moof", parser::children)
            .base_box("traf", parser::children)
            .full_box("tfdt", move |mut box_| {
                *saw_tfdt_c.borrow_mut() = true;

                let box_version = box_.version.unwrap();

                if box_version != 0 && box_version != 1 {
                    bail!("TFDT box version can only be 0 or 1.");
                }

                let tfdt_box = TfdtBox::new(&mut box_)?;
                *base_time_c.borrow_mut() = tfdt_box.base_media_decode_time;
                Ok(())
            })
            .full_box("tfhd", move |mut box_| {
                if box_.flags.is_none() {
                    bail!("TFHD box should have a valid flags value.");
                }

                let tfhd_box = TfhdBox::new(&mut box_)?;
                *default_duration_c.borrow_mut() = tfhd_box.default_sample_duration;
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

                let trun_box = TrunBox::new(&mut box_)?;
                *presentations_c.borrow_mut() = trun_box.sample_data;
                Ok(())
            })
            .base_box(
                "mdat",
                parser::alldata(move |data| {
                    if !*saw_tfdt.borrow() && !*saw_trun.borrow() {
                        bail!("Some required boxes (either TFDT or TRUN) are missing.");
                    }

                    let cues = Self::parse_mdat(
                        *base_time.borrow(),
                        *default_duration.borrow(),
                        period_start,
                        &presentations.borrow(),
                        &data,
                        timescale,
                    )?;
                    subtitles_c.borrow_mut().extend_cues(cues);
                    Ok(())
                }),
            )
            .parse(data, false, false)?;

        Ok(subtitles.take())
    }

    fn parse_mdat(
        base_time: u64,
        default_duration: Option<u32>,
        period_start: f32,
        presentations: &[TrunSample],
        raw_payload: &[u8],
        timescale: u32,
    ) -> Result<Vec<Cue>> {
        let mut cues = Vec::new();
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

            current_time = start_time + duration.unwrap_or_default() as u64;

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

                match payload_name.as_str() {
                    "vttc" => {
                        if payload_size > 8 {
                            payload = Some(reader.read_bytes_u8((payload_size - 8) as usize)?);
                        }
                    }
                    "vtte" => {
                        // It's a vtte, which is a vtt cue that is empty. Ignore any data that
                        // does exist.
                        reader.skip((payload_size - 8) as u64)?;
                    }
                    _ => {
                        // println!("Unknown box {} ! Skipping!", payload_name);
                        reader.skip((payload_size - 8) as u64)?;
                    }
                }

                if duration.is_some() {
                    if let Some(payload) = payload {
                        let cue = Self::parse_vttc(
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
                    || total_size <= presentation.sample_size.unwrap_or_default() as i32)
                {
                    bail!(
                        "The samples do not fit evenly into the sample sizes given in the TRUN box."
                    );
                };

                // If no sampleSize was specified, it's assumed that this presentation
                // corresponds to only a single cue.
                if !(presentation.sample_size.is_some()
                    && (total_size < presentation.sample_size.unwrap_or_default() as i32))
                {
                    break;
                }
            }
        }

        if reader.has_more_data() {
            bail!("MDAT which contain VTT cues and non-VTT data are not currently supported.");
        };

        Ok(cues.into_iter().flatten().collect())
    }

    /// Parses a vttc box into a cue.
    fn parse_vttc(data: &[u8], start_time: f32, end_time: f32) -> Result<Option<Cue>> {
        let payload = Rc::new(RefCell::new(String::new()));
        let settings = Rc::new(RefCell::new(String::new()));

        let payload_c = payload.clone();
        let settings_c = settings.clone();

        Mp4Parser::new()
            .base_box(
                "payl",
                parser::alldata(move |data| {
                    *payload_c.borrow_mut() = String::from_utf8(data)?;
                    Ok(())
                }),
            )
            // .base_box(
            //     "iden",
            //     parser::alldata(move |data| {
            //         *id_c.borrow_mut() = String::from_utf8(data)?;
            //         Ok(())
            //     }),
            // )
            .base_box(
                "sttg",
                parser::alldata(move |data| {
                    *settings_c.borrow_mut() = String::from_utf8(data)?;
                    Ok(())
                }),
            )
            .parse(data, false, false)?;

        let payload = payload.take();

        if !payload.is_empty() {
            return Ok(Some(Cue {
                payload,
                settings: settings.take(),
                start_time,
                end_time,
            }));
        }

        Ok(None)
    }
}
