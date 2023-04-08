/*
    REFERENCES
    ----------

    1. https://github.com/emarsden/dash-mpd-rs/blob/6ebdfb4759adbda8233b5b3520804e23ff86e7de/src/fetch.rs
    2. https://github.com/streamlink/streamlink/blob/781ef1fc92f215d0f3ec9a272fbe9f2cac122f08/src/streamlink/stream/dash_manifest.py
    2. https://github.com/nilaoda/N_m3u8DL-RE/blob/7bba10aa0d7adf7e79e0feec7327039681cb7bd4/src/N_m3u8DL-RE.Parser/Extractor/DASHExtractor2.cs

*/

use super::{DashUrl, Template};
use crate::playlist::{
    ByteRange, Key, KeyMethod, Map, MasterPlaylist, MediaPlaylist, MediaType, PlaylistType, Segment,
};
use anyhow::{anyhow, bail, Result};
use dash_mpd::MPD;
use reqwest::Url;
use std::collections::HashMap;

pub(crate) fn parse_as_master(mpd: &MPD, uri: &str) -> MasterPlaylist {
    let mut streams = vec![];

    if let Some(period) = mpd.periods.get(0) {
        let period_index = 0;
        // for (period_index, period) in mpd.periods.iter().enumerate() {
        for (adaptation_index, adaptation_set) in period.adaptations.iter().enumerate() {
            for (representation_index, representation) in
                adaptation_set.representations.iter().enumerate()
            {
                // https://dashif.org/codecs/introduction
                let codecs = if representation.codecs.is_some() {
                    representation.codecs.clone()
                } else if adaptation_set.codecs.is_some() {
                    adaptation_set.codecs.clone()
                } else {
                    None
                };

                let mime_type = if representation.mimeType.is_some() {
                    representation.mimeType.clone()
                } else if representation.contentType.is_some() {
                    representation.contentType.clone()
                } else if adaptation_set.mimeType.is_some() {
                    adaptation_set.mimeType.clone()
                } else if adaptation_set.contentType.is_some() {
                    adaptation_set.contentType.clone()
                } else {
                    None
                };

                let mut media_type = if let Some(mime_type) = &mime_type {
                    match mime_type.as_str() {
                        "application/ttml+xml" | "application/x-sami" => MediaType::Subtitles,
                        x if x.starts_with("audio") => MediaType::Audio,
                        x if x.starts_with("text") => MediaType::Subtitles,
                        x if x.starts_with("video") => MediaType::Video,
                        _ => MediaType::Undefined,
                    }
                } else {
                    MediaType::Undefined
                };

                if let Some(codecs) = &codecs {
                    media_type = match codecs.as_str() {
                        "wvtt" | "stpp" => MediaType::Subtitles,
                        x if x.starts_with("stpp.") => MediaType::Subtitles,
                        _ => media_type,
                    };
                }

                // if let Some(role) = &representation.role {
                //     if let Some(value) = &role.value {
                //         if value == "subtitle" {
                //             media_type = MediaType::Subtitles;
                //         }
                //     }
                // }

                streams.push(MediaPlaylist {
                    bandwidth: representation.bandwidth,
                    channels: if let Some(value) = representation
                        .AudioChannelConfiguration
                        .get(0)
                        .map(|x| x.value.as_ref().map(|y| y.parse::<f32>().ok()))
                        .flatten()
                        .flatten()
                    {
                        Some(value)
                    } else if let Some(value) = adaptation_set
                        .AudioChannelConfiguration
                        .get(0)
                        .map(|x| x.value.as_ref().map(|y| y.parse::<f32>().ok()))
                        .flatten()
                        .flatten()
                    {
                        Some(value)
                    } else {
                        None
                    },
                    codecs,
                    extension: mime_type
                        .as_ref()
                        .map(|x| x.split_terminator('/').nth(1).map(|x| x.to_owned()))
                        .flatten(),
                    frame_rate: if representation.frameRate.is_some() {
                        parse_frame_rate(&representation.frameRate)
                    } else if adaptation_set.frameRate.is_some() {
                        parse_frame_rate(&adaptation_set.frameRate)
                    } else {
                        None
                    },
                    i_frame: false, // Cannot be comment here
                    language: adaptation_set.lang.clone(),
                    live: if let Some(mpdtype) = &mpd.mpdtype {
                        mpdtype == "dynamic"
                    } else {
                        false
                    },
                    media_type,
                    playlist_type: PlaylistType::Dash,
                    resolution: if let (Some(width), Some(height)) =
                        (representation.width, representation.height)
                    {
                        Some((width, height))
                    } else {
                        None
                    },
                    segments: vec![], // Cannot be comment here
                    uri: DashUrl::new(period_index, adaptation_index, representation_index)
                        .to_string(),
                });
            }
        }
    }

    MasterPlaylist {
        playlist_type: PlaylistType::Dash,
        uri: uri.to_owned(),
        streams,
    }
}

pub(crate) fn push_segments(mpd: &MPD, playlist: &mut MediaPlaylist, base_url: &str) -> Result<()> {
    let location = playlist.uri.parse::<DashUrl>().map_err(|x| anyhow!(x))?;

    for (_period_index, period) in mpd.periods.iter().enumerate() {
        for (adaptation_index, adaptation_set) in period.adaptations.iter().enumerate() {
            for (representation_index, representation) in
                adaptation_set.representations.iter().enumerate()
            {
                if adaptation_index == location.adaptation_set
                    && representation_index == location.representation
                {
                    let mut period_duration_secs = 0.0;

                    if let Some(duration) = &mpd.mediaPresentationDuration {
                        period_duration_secs = duration.as_secs_f32();
                    }

                    if let Some(duration) = &period.duration {
                        period_duration_secs = duration.as_secs_f32();
                    }

                    let mut base_url = base_url.parse::<Url>().unwrap();

                    if let Some(mpd_baseurl) = mpd.base_url.get(0).map(|x| x.base.as_ref()) {
                        base_url = base_url.join(mpd_baseurl)?;
                    }

                    if let Some(period_baseurl) = period.BaseURL.get(0).map(|x| x.base.as_ref()) {
                        base_url = base_url.join(period_baseurl)?;
                    }

                    if let Some(adaptation_set_baseurl) =
                        adaptation_set.BaseURL.get(0).map(|x| x.base.as_ref())
                    {
                        base_url = base_url.join(adaptation_set_baseurl)?;
                    }

                    if let Some(representation_baseurl) =
                        representation.BaseURL.get(0).map(|x| x.base.as_ref())
                    {
                        base_url = base_url.join(representation_baseurl)?;
                    }

                    let mut init_map = None;

                    let rid = if let Some(id) = &representation.id {
                        id.to_owned()
                    } else {
                        bail!("missing @id on representation node.");
                    };

                    let mut template_vars = HashMap::from([("RepresentationID".to_owned(), rid)]);

                    if let Some(bandwidth) = &representation.bandwidth {
                        template_vars.insert("Bandwidth".to_owned(), bandwidth.to_string());
                    }

                    let mut template = Template::new(template_vars);

                    // Now the 6 possible addressing modes:
                    // (1) SegmentList
                    // (2) SegmentTemplate+SegmentTimeline
                    // (3) SegmentTemplate@duration
                    // (4) SegmentTemplate@index
                    // (5) SegmentBase@indexRange
                    // (6) Plain BaseURL

                    // Though SegmentBase and SegmentList addressing modes are supposed to be
                    // mutually exclusive, some manifests in the wild use both. So we try to work
                    // around the brokenness.

                    // (1) AdaptationSet>SegmentList
                    if let Some(segment_list) = &adaptation_set.SegmentList {
                        if let Some(initialization) = &segment_list.Initialization {
                            let byte_range = parse_range(&initialization.range);

                            if let Some(source_url) = &initialization.sourceURL {
                                init_map = Some(Map {
                                    byte_range,
                                    uri: base_url.join(&template.resolve(source_url))?.to_string(),
                                });
                            } else {
                                init_map = Some(Map {
                                    byte_range,
                                    uri: base_url.to_string(),
                                });
                            }
                        }

                        for segment_url in &segment_list.segment_urls {
                            // We are ignoring SegmentURL@indexRange
                            let byte_range = parse_range(&segment_url.mediaRange);

                            if let Some(media) = &segment_url.media {
                                playlist.segments.push(Segment {
                                    byte_range,
                                    uri: base_url.join(media)?.to_string(),
                                    ..Default::default()
                                });
                            } else if !adaptation_set.BaseURL.is_empty() {
                                playlist.segments.push(Segment {
                                    byte_range,
                                    uri: base_url.to_string(),
                                    ..Default::default()
                                });
                            }
                        }
                    }

                    // (1) Representation>SegmentList
                    if let Some(segment_list) = &representation.SegmentList {
                        if let Some(initialization) = &segment_list.Initialization {
                            let byte_range = parse_range(&initialization.range);

                            if let Some(source_url) = &initialization.sourceURL {
                                init_map = Some(Map {
                                    byte_range,
                                    uri: base_url.join(&template.resolve(source_url))?.to_string(),
                                });
                            } else {
                                init_map = Some(Map {
                                    byte_range,
                                    uri: base_url.to_string(),
                                });
                            }
                        }

                        for segment_url in &segment_list.segment_urls {
                            // We are ignoring SegmentURL@indexRange
                            let byte_range = parse_range(&segment_url.mediaRange);

                            if let Some(media) = &segment_url.media {
                                playlist.segments.push(Segment {
                                    byte_range,
                                    uri: base_url.join(media)?.to_string(),
                                    ..Default::default()
                                });
                            } else if !representation.BaseURL.is_empty() {
                                playlist.segments.push(Segment {
                                    byte_range,
                                    uri: base_url.to_string(),
                                    ..Default::default()
                                });
                            }
                        }
                    } else if representation.SegmentTemplate.is_some()
                        || adaptation_set.SegmentTemplate.is_some()
                    {
                        let segment_template = representation
                            .SegmentTemplate
                            .as_ref()
                            .or(adaptation_set.SegmentTemplate.as_ref())
                            .unwrap();

                        if let Some(initialization) = &segment_template.initialization {
                            init_map = Some(Map {
                                byte_range: None,
                                uri: base_url
                                    .join(&template.resolve(initialization))?
                                    .to_string(),
                            });
                        }

                        // (2) SegmentTemplate+SegmentTimeline (explicit addressing)
                        if let Some(segment_timeline) = &segment_template.SegmentTimeline {
                            if let Some(media) = &segment_template.media {
                                let media = template.resolve(media);
                                let timescale = segment_template.timescale.unwrap_or(1) as f32;
                                let mut segment_time = 0;
                                let mut number = segment_template.startNumber.unwrap_or(1);

                                for s in &segment_timeline.segments {
                                    if let Some(t) = s.t {
                                        segment_time = t;
                                    }

                                    template.insert("Time", segment_time.to_string());
                                    template.insert("Number", number.to_string());

                                    playlist.segments.push(Segment {
                                        duration: s.d as f32 / timescale,
                                        uri: base_url.join(&template.resolve(&media))?.to_string(),
                                        ..Default::default()
                                    });

                                    number += 1;

                                    // let mut repeat_count = s.r.unwrap_or(0);
                                    // if repeat_count < 0 {
                                    //     repeat_count = ((period_duration_secs * timescale / s.d as f32) - 1.0) as i64;
                                    // }
                                    // for _ in 0..repeat_count {}

                                    if let Some(r) = s.r {
                                        let mut count = 0;
                                        // FIXME - Perhaps we also need to account for startTime?
                                        let end_time = period_duration_secs * timescale;

                                        loop {
                                            count += 1;
                                            // Exit from the loop after @r iterations (if @r is
                                            // positive). A negative value of the @r attribute indicates
                                            // that the duration indicated in @d attribute repeats until
                                            // the start of the next S element, the end of the Period or
                                            // until the next MPD update.
                                            if r >= 0 {
                                                if count > r {
                                                    break;
                                                }
                                            } else if segment_time as f32 > end_time {
                                                break;
                                            }

                                            segment_time += s.d;

                                            template.insert("Time", segment_time.to_string());
                                            template.insert("Number", number.to_string());

                                            playlist.segments.push(Segment {
                                                duration: s.d as f32 / timescale,
                                                uri: base_url
                                                    .join(&template.resolve(&media))?
                                                    .to_string(),
                                                ..Default::default()
                                            });

                                            number += 1;
                                        }
                                    }

                                    segment_time += s.d;
                                }
                            } else {
                                bail!("SegmentTimeline without a media attribute.");
                            }
                        } else {
                            // (3) SegmentTemplate@duration || (4) SegmentTemplate@index (simple addressing)
                            if let Some(media) = &segment_template.media {
                                let media = template.resolve(media);
                                let timescale = segment_template.timescale.unwrap_or(1) as f32;
                                let mut duration = 0.0;

                                if let Some(x) = period.duration {
                                    duration = x.as_secs_f32();
                                }

                                if let Some(x) = segment_template.duration {
                                    duration = x as f32 / timescale;
                                }

                                if duration == 0.0 {
                                    bail!("Representation is missing SegmentTemplate @duration attribute.");
                                }

                                let total_number =
                                    (period_duration_secs / duration).ceil() as usize;
                                let mut number = segment_template.startNumber.unwrap_or(1);

                                for _ in 1..=total_number {
                                    template.insert("Number", number.to_string());

                                    playlist.segments.push(Segment {
                                        duration,
                                        uri: base_url.join(&template.resolve(&media))?.to_string(),
                                        ..Default::default()
                                    });

                                    number += 1;
                                }
                            }
                        }
                    } else if let Some(segment_base) = &representation.SegmentBase {
                        // (5) SegmentBase@indexRange
                        // The SegmentBase@indexRange attribute points to a byte range in the media
                        // file that contains index information (an sidx box for MPEG files, or a
                        // Cues entry for a DASH-WebM stream). To be fully compliant, we should
                        // download and parse these (for example using the sidx crate) then download
                        // the referenced content segments. In practice, it seems that the
                        // indexRange information is mostly provided by DASH encoders to allow
                        // clients to rewind and fast-foward a stream, and is not necessary if we
                        // download the full content specified by BaseURL.
                        //
                        // Our strategy: if there is a SegmentBase > Initialization > SourceURL
                        // node, download that first, respecting the byte range if it is specified.
                        // Otherwise, download the full content specified by the BaseURL for this
                        // segment (ignoring any indexRange attributes).
                        //
                        // https://github.com/shaka-project/shaka-player/blob/main/lib/dash/segment_base.js
                        // https://github.com/shaka-project/shaka-player/blob/main/lib/media/mp4_segment_index_parser.js

                        if let Some(initialization) = &segment_base.initialization {
                            let byte_range = parse_range(&initialization.range);

                            if let Some(source_url) = &initialization.sourceURL {
                                init_map = Some(Map {
                                    byte_range,
                                    uri: base_url.join(&template.resolve(source_url))?.to_string(),
                                });
                            }
                        }

                        playlist.segments.push(Segment {
                            uri: base_url.to_string(),
                            ..Default::default()
                        });
                    } else if playlist.segments.is_empty() && !representation.BaseURL.is_empty() {
                        // (6) Plain BaseURL
                        playlist.segments.push(Segment {
                            duration: period_duration_secs,
                            uri: base_url.to_string(),
                            ..Default::default()
                        });
                    }

                    if playlist.segments.is_empty() {
                        bail!("no usable addressing mode identified for representation.");
                    }

                    if let Some(first_segment) = playlist.segments.get_mut(0) {
                        let mut encryption_type = KeyMethod::None;
                        let mut default_kid = None;

                        for content_protection in &representation.ContentProtection {
                            if default_kid.is_none() && content_protection.default_KID.is_some() {
                                default_kid = content_protection.default_KID.clone();
                            }
                            
                            // content_protection.value = "cenc" | "cbcs"
                            if encryption_type == KeyMethod::None
                                && content_protection.value.is_some()
                            {
                                encryption_type = KeyMethod::Cenc;
                            }
                        }

                        if encryption_type == KeyMethod::None || default_kid.is_none() {
                            for content_protection in &adaptation_set.ContentProtection {
                                if default_kid.is_none() && content_protection.default_KID.is_some()
                                {
                                    default_kid = content_protection.default_KID.clone();
                                }

                                if encryption_type == KeyMethod::None
                                    && content_protection.value.is_some()
                                {
                                    encryption_type = KeyMethod::Cenc;
                                }
                            }
                        }

                        default_kid = default_kid.map(|x| x.to_lowercase());

                        first_segment.key = match encryption_type {
                            KeyMethod::None => None,
                            x => Some(Key {
                                default_kid,
                                iv: None,
                                key_format: None,
                                method: x,
                                uri: "dash://encryption-key".to_owned(),
                            }),
                        };

                        first_segment.map = init_map;
                    }
                }
            }
        }
    }

    // if let Some(segment_template) = representation.segment_template(adaptation_set) {
    //     let mut template_resolver = TemplateResolver::new(representation.template_vars());

    //     if let Some(initialization) = &segment_template.initialization {
    //         init_map = Some(playlist::Map {
    //             byte_range: None,
    //             uri: template_resolver.resolve(baseurl.join(initialization)?.as_str()),
    //         });

    //         let mut start_number = segment_template.start_number();
    //         let timescale = segment_template.timescale();

    //         if let Some(segment_timeline) = &segment_template.segment_timeline {
    //             let mut current_time = 0;

    //             for s in &segment_timeline.s {
    //                 if let Some(t) = &s.t {
    //                     current_time = *t;
    //                 }

    //                 template_resolver.insert("Time", current_time.to_string());
    //                 template_resolver.insert("Number", start_number.to_string());

    //                 playlist.segments.push(playlist::Segment {
    //                     duration: s.d as f32 / timescale,
    //                     uri: template_resolver.resolve(
    //                         baseurl
    //                             .join(segment_template.media.as_ref().unwrap())?
    //                             .as_str(),
    //                     ),
    //                     ..Default::default()
    //                 });

    //                 start_number += 1;

    //                 let mut repeat_count = s.r.unwrap_or(0);

    //                 if repeat_count < 0 {
    //                     repeat_count = ((mpd_duration * timescale / s.d as f32) - 1.0) as i64;
    //                 }

    //                 for _ in 0..repeat_count {
    //                     current_time += s.d;

    //                     template_resolver.insert("Time", current_time.to_string());
    //                     template_resolver.insert("Number", start_number.to_string());

    //                     playlist.segments.push(playlist::Segment {
    //                         duration: s.d as f32 / timescale,
    //                         uri: template_resolver.resolve(
    //                             baseurl
    //                                 .join(segment_template.media.as_ref().unwrap())?
    //                                 .as_str(),
    //                         ),
    //                         ..Default::default()
    //                     });

    //                     start_number += 1;
    //                 }

    //                 current_time += s.d;
    //             }
    //         } else {
    //             let duration = segment_template.duration();
    //             let segment_duration = duration / timescale;
    //             let mut total = (mpd_duration * timescale / duration).ceil() as usize;

    //             if total == 0 && mpd.live() {
    //                 let now = if let Some(publish_time) = &mpd.publish_time {
    //                     chrono::DateTime::parse_from_rfc3339(publish_time).unwrap()
    //                 } else {
    //                     chrono::Local::now().into()
    //                 };

    //                 let available_time = chrono::DateTime::parse_from_rfc3339(
    //                     mpd.availability_start_time.as_ref().unwrap(),
    //                 )
    //                 .unwrap();
    //                 let ts = now - available_time;
    //                 let update_ts =
    //                     iso8601_duration_to_seconds(mpd.time_shift_buffer_depth.as_ref().unwrap())
    //                         .unwrap();
    //                 start_number +=
    //                     ((ts.num_seconds() as f32 - update_ts) * timescale / duration) as usize;
    //                 total = (update_ts * timescale / duration) as usize;
    //             }

    //             for i in start_number..(start_number + total) {
    //                 template_resolver.insert("Number", i.to_string());

    //                 playlist.segments.push(playlist::Segment {
    //                     duration: segment_duration,
    //                     uri: template_resolver.resolve(
    //                         baseurl
    //                             .join(segment_template.media.as_ref().unwrap())?
    //                             .as_str(),
    //                     ),
    //                     ..Default::default()
    //                 });
    //             }
    //         }
    //     }
    // }

    Ok(())
}

fn parse_frame_rate(frame_rate: &Option<String>) -> Option<f32> {
    frame_rate
        .as_ref()
        .map(|frame_rate| {
            if frame_rate.contains('/') {
                let splitted_frame_rate = frame_rate
                    .split_terminator('/')
                    .filter_map(|x| x.parse::<f32>().ok())
                    .collect::<Vec<f32>>();

                if let (Some(upper), Some(lower)) =
                    (splitted_frame_rate.get(0), splitted_frame_rate.get(1))
                {
                    Some(upper / lower)
                } else {
                    panic!("could'nt parse \"{}\" frame rate", frame_rate);
                }
            } else {
                frame_rate.parse::<f32>().ok()
            }
        })
        .flatten()
}

fn parse_range(range: &Option<String>) -> Option<ByteRange> {
    range.as_ref().map(|range| {
        let splitted_range = range
            .split_terminator('-')
            .filter_map(|x| x.parse::<u64>().ok())
            .collect::<Vec<u64>>();

        if let (Some(length), offset) = (splitted_range.get(0), splitted_range.get(1)) {
            ByteRange {
                length: *length,
                offset: offset.map(|x| (*x - *length) + 1),
            }
        } else {
            panic!("could'nt convert \"{}\" range to byte range", range);
        }
    })
}

// // Sec-Fetch-Mode: navigate
// // Upgrade-Insecure-Requests: 1
// /*
// A manifest may use a data URL (RFC 2397) to embed media content such as the
// initialization segment directly in the manifest (recommended by YouTube for live
// streaming, but uncommon in practice).
//  */
// if url.scheme() == "data" {
//     let us = &url.to_string();
//     let du = DataUrl::process(us)
//         .map_err(|_| DashMpdError::Parsing(String::from("parsing data URL")))?;
//     if du.mime_type().type_ != "audio" {
//         return Err(DashMpdError::UnhandledMediaStream(
//             String::from("expecting audio content in data URL")));
//     }
//     let (body, _fragment) = du.decode_to_vec()
//         .map_err(|_| DashMpdError::Parsing(String::from("decoding data URL")))?;
//     if downloader.verbosity > 2 {
//         println!("Audio segment data URL -> {} octets", body.len());
//     }
//     if let Err(e) = tmpfile_audio.write_all(&body) {
//         log::error!("Unable to write DASH audio data: {e:?}");
//         return Err(DashMpdError::Io(e, String::from("writing DASH audio data")));
//     }
//     have_audio = true;
