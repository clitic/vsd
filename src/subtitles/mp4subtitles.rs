use super::{mp4parser, ttml};
use super::{Cue, MP4Parser, Reader, Subtitles};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

pub struct MP4Subtitles {
    saw_wvtt: bool,
    saw_stpp: bool,
    timescale: u32,
}

impl MP4Subtitles {
    pub fn from_init(init: &[u8]) -> Result<Self, String> {
        let timescale = Arc::new(Mutex::new(0));
        let saw_wvtt = Arc::new(Mutex::new(false));
        let saw_stpp = Arc::new(Mutex::new(false));

        let timescale_c = timescale.clone();
        let saw_wvtt_c = saw_wvtt.clone();
        let saw_stpp_c = saw_stpp.clone();

        MP4Parser::default()
            .basic("moov", Arc::new(mp4parser::children))
            .basic("trak", Arc::new(mp4parser::children))
            .basic("mdia", Arc::new(mp4parser::children))
            .full(
                "mdhd",
                Arc::new(move |mut _box| {
                    if !(_box.version == 0 || _box.version == 1) {
                        return Err("MDHD version can only be 0 or 1".to_owned());
                    }

                    *timescale_c.lock().unwrap() = _box.reader.parse_mdhd(_box.version);
                    Ok(())
                }),
            )
            .basic("minf", Arc::new(mp4parser::children))
            .basic("stbl", Arc::new(mp4parser::children))
            .full("stsd", Arc::new(mp4parser::sample_description))
            .basic(
                "wvtt",
                Arc::new(move |_box| {
                    *saw_wvtt_c.lock().unwrap() = true;
                    Ok(())
                }),
            )
            .basic(
                "stpp",
                Arc::new(move |_box| {
                    *saw_stpp_c.lock().unwrap() = true;
                    Ok(())
                }),
            )
            .parse(init, None, None)?;

        let saw_wvtt = *saw_wvtt.lock().unwrap();
        let saw_stpp = *saw_stpp.lock().unwrap();
        let timescale = *timescale.lock().unwrap();

        if (saw_wvtt && timescale != 0) || saw_stpp {
            Ok(Self {
                saw_wvtt,
                saw_stpp,
                timescale,
            })
        } else if timescale == 0 {
            Err("Missing timescale for VTT content!".to_owned())
        } else {
            Err("Missing wvtt/stpp box in init".to_owned())
        }
    }

    pub fn to_subtitles(&self, segments: &[Vec<u8>]) -> Result<Subtitles, String> {
        let mut cues: Vec<Cue> = vec![];

        if self.saw_wvtt {
            for data_seg in segments {
                let saw_tfdt = Arc::new(Mutex::new(false));
                let saw_trun = Arc::new(Mutex::new(false));
                let saw_mdat = Arc::new(Mutex::new(false));
                let raw_payload = Arc::new(Mutex::new(None));
                let base_time = Arc::new(Mutex::new(0_u64));
                let default_duration = Arc::new(Mutex::new(0));
                let presentations = Arc::new(Mutex::new(vec![]));

                let saw_tfdt_c = saw_tfdt.clone();
                let saw_trun_c = saw_trun.clone();
                let saw_mdat_c = saw_mdat.clone();
                let raw_payload_c = raw_payload.clone();
                let base_time_c = base_time.clone();
                let default_duration_c = default_duration.clone();
                let presentations_c = presentations.clone();

                MP4Parser::default()
                    .basic("moof", Arc::new(mp4parser::children))
                    .basic("traf", Arc::new(mp4parser::children))
                    .full(
                        "tfdt",
                        Arc::new(move |mut _box| {
                            if !(_box.version == 0 || _box.version == 1) {
                                return Err("TFDT version can only be 0 or 1".to_owned());
                            }

                            *saw_tfdt_c.lock().unwrap() = true;
                            *base_time_c.lock().unwrap() = _box.reader.parse_tfdt(_box.version);
                            Ok(())
                        }),
                    )
                    .full(
                        "tfhd",
                        Arc::new(move |mut _box| {
                            if _box.flags == 1000 {
                                return Err("A TFHD box should have a valid flags value".to_owned());
                            }

                            *default_duration_c.lock().unwrap() =
                                _box.reader.parse_tfhd(_box.flags).default_sample_duration;

                            Ok(())
                        }),
                    )
                    .full(
                        "trun",
                        Arc::new(move |mut _box| {
                            if _box.version == 1000 {
                                return Err(
                                    "A TRUN box should have a valid version value".to_owned()
                                );
                            }

                            if _box.flags == 1000 {
                                return Err("A TRUN box should have a valid flags value".to_owned());
                            }

                            *saw_trun_c.lock().unwrap() = true;
                            *presentations_c.lock().unwrap() =
                                _box.reader.parse_trun(_box.version, _box.flags).sample_data;
                            Ok(())
                        }),
                    )
                    .basic(
                        "mdat",
                        mp4parser::alldata(Arc::new(move |data| {
                            let mut saw_mdat_once = saw_mdat_c.lock().unwrap();

                            if *saw_mdat_once {
                                return Err(
                                "VTT cues in mp4 with multiple MDAT are not currently supported"
                                    .to_owned(),
                            );
                            }

                            *saw_mdat_once = true;
                            *raw_payload_c.lock().unwrap() = Some(data);
                            Ok(())
                        })),
                    )
                    .parse(data_seg, Some(false), None)?;

                let saw_tfdt = *saw_tfdt.lock().unwrap();
                let saw_trun = *saw_trun.lock().unwrap();
                let saw_mdat = *saw_mdat.lock().unwrap();
                let raw_payload = raw_payload.lock().unwrap().clone();
                let base_time = *base_time.lock().unwrap();
                let default_duration = *default_duration.lock().unwrap();
                let presentations = presentations.lock().unwrap().clone();

                if !saw_mdat && !saw_tfdt && !saw_trun {
                    return Err("A required box is missing".to_owned());
                }

                let mut current_time = base_time.clone();

                let mut reader = Reader {
                    cursor: Cursor::new(raw_payload.unwrap()),
                };

                for presentation in presentations {
                    let duration = if presentation.sample_duration == 0 {
                        default_duration
                    } else {
                        presentation.sample_duration
                    };

                    let start_time = if presentation.sample_composition_time_offset != 0 {
                        base_time + presentation.sample_composition_time_offset as u64
                    } else {
                        current_time
                    };

                    current_time = start_time + duration as u64;

                    let mut total_size = 0;

                    loop {
                        let payload_size = reader.read_u32() as i32;
                        total_size += payload_size;

                        let payload_type = reader.read_u32();
                        let payload_name = mp4parser::type_to_string(payload_type);

                        let mut payload = None;
                        if payload_name == "vttc" {
                            if payload_size > 8 {
                                payload = Some(reader.read_bytes((payload_size - 8) as usize));
                            }
                        } else if payload_name == "vtte" {
                            let _ = reader.read_bytes((payload_size - 8) as usize);
                        } else {
                            reader.read_bytes((payload_size - 8) as usize);
                        }

                        if duration != 0 {
                            if let Some(payload) = payload {
                                let cue = Cue::parse_vttc(
                                    &payload,
                                    start_time as f32 / self.timescale as f32,
                                    current_time as f32 / self.timescale as f32,
                                );

                                if let Some(cue) = cue {
                                    let mut index = None;

                                    for (i, s) in cues.iter().enumerate() {
                                        if s.end_time == cue.start_time
                                            && s.settings == cue.settings
                                            && s.payload == cue.payload
                                        {
                                            index = Some(i);
                                        }
                                    }

                                    if let Some(index) = index {
                                        cues.get_mut(index).unwrap().end_time = cue.end_time;
                                    } else {
                                        cues.push(cue);
                                    }
                                }
                            }
                        } else {
                            return Err(
                                "WVTT sample duration unknown, and no default found!".to_owned()
                            );
                        }

                        if !(presentation.sample_size == 0
                            || total_size <= presentation.sample_size as i32)
                        {
                            return Err("The samples do not fit evenly into the sample sizes given in the TRUN box!".to_owned());
                        }

                        if !(presentation.sample_size != 0
                            && (total_size < presentation.sample_size as i32))
                        {
                            break;
                        }
                    }

                    // if reader.has_more_data() {
                    //     return Err(
                    //         "MDAT which contain VTT cues and non-VTT data are not currently supported!"
                    //             .to_owned(),
                    //     );
                    // }
                }
            }
        } else if self.saw_stpp {
            for data_seg in segments {
                // let saw_mdat = Arc::new(Mutex::new(false));
                let raw_payload = Arc::new(Mutex::new(None));

                // let saw_mdat_c = saw_mdat.clone();
                let raw_payload_c = raw_payload.clone();

                MP4Parser::default()
                    .basic(
                        "mdat",
                        mp4parser::alldata(Arc::new(move |data| {
                            // *saw_mdat_c = true;
                            *raw_payload_c.lock().unwrap() = Some(data);
                            Ok(())
                        })),
                    )
                    .parse(data_seg, Some(false), None)?;

                // let saw_mdat = *saw_mdat.lock().unwrap();
                let raw_payload = raw_payload.lock().unwrap().clone();

                if let Some(xml) = &raw_payload {
                    cues.append(
                        &mut quick_xml::de::from_reader::<_, ttml::TT<ttml::DivAttributes>>(
                            xml.as_slice(),
                        )
                        .unwrap()
                        .to_cues(
                            &quick_xml::de::from_reader::<_, ttml::TT<ttml::DivValue>>(
                                xml.as_slice(),
                            )
                            .unwrap(),
                        ),
                    );
                }
            }

            // return Ok(Subtitles::new(cues).merge());
        }

        Ok(Subtitles::new(cues))
    }

    pub fn is_vtt(&self) -> bool {
        self.saw_wvtt
    }

    pub fn is_ttml(&self) -> bool {
        self.saw_stpp
    }
}
