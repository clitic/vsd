/*
    REFERENCES
    ----------

    1. https://github.com/emarsden/dash-mpd-rs/blob/7e985069fd95fd5d9993b7610c28228d2448aea7/src/fetch.rs#L2428-L2870

*/

use super::{DashUrl, Template};
use crate::playlist::{
    Key, KeyMethod, Map, MasterPlaylist, MediaPlaylist, MediaType, PlaylistType, Range, Segment,
};
use anyhow::{Result, anyhow, bail};
use dash_mpd::MPD;
use reqwest::{Client, Url, header};
use std::collections::HashMap;

pub(crate) fn parse_as_master(mpd: &MPD, uri: &str) -> MasterPlaylist {
    let mut streams = vec![];

    if let Some(period) = mpd.periods.first() {
        let period_index = 0;
        for (adaptation_index, adaptation_set) in period.adaptations.iter().enumerate() {
            for (representation_index, representation) in
                adaptation_set.representations.iter().enumerate()
            {
                // https://dashif.org/codecs/introduction
                let codecs = representation
                    .codecs
                    .clone()
                    .or(adaptation_set.codecs.clone());

                let mime_type = representation
                    .mimeType
                    .clone()
                    .or(adaptation_set.mimeType.clone())
                    .or(representation.contentType.clone())
                    .or(adaptation_set.contentType.clone());

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

                if media_type == MediaType::Undefined
                    && let Some(codecs) = &codecs
                {
                    media_type = match codecs.as_str() {
                        "wvtt" | "stpp" => MediaType::Subtitles,
                        x if x.starts_with("stpp.") => MediaType::Subtitles,
                        _ => media_type,
                    };
                }

                streams.push(MediaPlaylist {
                    bandwidth: representation.bandwidth,
                    channels: representation
                        .AudioChannelConfiguration
                        .first()
                        .and_then(|x| x.value.as_ref().map(|y| y.parse::<f32>().ok()))
                        .flatten()
                        .or(adaptation_set
                            .AudioChannelConfiguration
                            .first()
                            .and_then(|x| x.value.as_ref().map(|y| y.parse::<f32>().ok()))
                            .flatten()),
                    codecs,
                    extension: mime_type
                        .as_ref()
                        .and_then(|x| x.split_once('/').map(|x| x.1.to_owned())),
                    frame_rate: if representation.frameRate.is_some() {
                        parse_frame_rate(&representation.frameRate)
                    } else if adaptation_set.frameRate.is_some() {
                        parse_frame_rate(&adaptation_set.frameRate)
                    } else {
                        None
                    },
                    id: String::new(), // Cannot be comment here
                    i_frame: false,    // Cannot be comment here
                    language: adaptation_set.lang.clone(),
                    live: if let Some(mpdtype) = &mpd.mpdtype {
                        mpdtype == "dynamic"
                    } else {
                        false
                    },
                    media_sequence: 0,
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

pub(crate) async fn push_segments(
    mpd: &MPD,
    playlist: &mut MediaPlaylist,
    base_url: &str,
    client: &Client,
    query: &HashMap<String, String>,
) -> Result<()> {
    let location = playlist.uri.parse::<DashUrl>().map_err(|x| anyhow!(x))?;

    for period in mpd.periods.iter() {
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

                    if let Some(mpd_baseurl) = mpd.base_url.first().map(|x| x.base.as_ref()) {
                        base_url = base_url.join(mpd_baseurl)?;
                    }

                    if let Some(period_baseurl) = period.BaseURL.first().map(|x| x.base.as_ref()) {
                        base_url = base_url.join(period_baseurl)?;
                    }

                    if let Some(adaptation_set_baseurl) =
                        adaptation_set.BaseURL.first().map(|x| x.base.as_ref())
                    {
                        base_url = base_url.join(adaptation_set_baseurl)?;
                    }

                    if let Some(representation_baseurl) =
                        representation.BaseURL.first().map(|x| x.base.as_ref())
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
                    // (1.1) AdaptationSet>SegmentList
                    // (1.2) Representation>SegmentList
                    // ( 2 ) SegmentTemplate+SegmentTimeline
                    // ( 3 ) SegmentTemplate@duration
                    // ( 4 ) SegmentTemplate@index
                    // ( 5 ) SegmentBase@indexRange
                    // ( 6 ) Plain BaseURL

                    // Though SegmentBase and SegmentList addressing modes are supposed to be
                    // mutually exclusive, some manifests in the wild use both. So we try to work
                    // around the brokenness.

                    // (1.1) AdaptationSet>SegmentList
                    if let Some(segment_list) = &adaptation_set.SegmentList {
                        if let Some(initialization) = &segment_list.Initialization {
                            let byte_range = parse_range(&initialization.range);

                            if let Some(source_url) = &initialization.sourceURL {
                                init_map = Some(Map {
                                    range: byte_range,
                                    uri: base_url.join(&template.resolve(source_url))?.to_string(),
                                });
                            } else {
                                init_map = Some(Map {
                                    range: byte_range,
                                    uri: base_url.to_string(),
                                });
                            }
                        }

                        for segment_url in &segment_list.segment_urls {
                            // We are ignoring SegmentURL@indexRange
                            let byte_range = parse_range(&segment_url.mediaRange);

                            if let Some(media) = &segment_url.media {
                                playlist.segments.push(Segment {
                                    range: byte_range,
                                    uri: base_url.join(media)?.to_string(),
                                    ..Default::default()
                                });
                            } else if !adaptation_set.BaseURL.is_empty() {
                                playlist.segments.push(Segment {
                                    range: byte_range,
                                    uri: base_url.to_string(),
                                    ..Default::default()
                                });
                            }
                        }
                    }

                    // (1.2) Representation>SegmentList
                    if let Some(segment_list) = &representation.SegmentList {
                        if let Some(initialization) = &segment_list.Initialization {
                            let byte_range = parse_range(&initialization.range);

                            if let Some(source_url) = &initialization.sourceURL {
                                init_map = Some(Map {
                                    range: byte_range,
                                    uri: base_url.join(&template.resolve(source_url))?.to_string(),
                                });
                            } else {
                                init_map = Some(Map {
                                    range: byte_range,
                                    uri: base_url.to_string(),
                                });
                            }
                        }

                        for segment_url in &segment_list.segment_urls {
                            // We are ignoring SegmentURL@indexRange
                            let byte_range = parse_range(&segment_url.mediaRange);

                            if let Some(media) = &segment_url.media {
                                playlist.segments.push(Segment {
                                    range: byte_range,
                                    uri: base_url.join(media)?.to_string(),
                                    ..Default::default()
                                });
                            } else if !representation.BaseURL.is_empty() {
                                playlist.segments.push(Segment {
                                    range: byte_range,
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
                                range: None,
                                uri: base_url
                                    .join(&template.resolve(initialization))?
                                    .to_string(),
                            });
                        }

                        // (2) SegmentTemplate+SegmentTimeline (explicit addressing)
                        if let Some(segment_timeline) = &segment_template.SegmentTimeline {
                            if segment_template.media.is_none() {
                                bail!("SegmentTimeline without a media attribute.");
                            }

                            let media = template.resolve(segment_template.media.as_ref().unwrap());
                            let mut number = segment_template.startNumber.unwrap_or(1);
                            let mut segment_time = 0;
                            let timescale = segment_template.timescale.unwrap_or(1) as f32;

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
                        } else if let Some(media) = &segment_template.media {
                            // (3) SegmentTemplate@duration || (4) SegmentTemplate@index (simple addressing)
                            let mut segment_duration = -1.0;
                            let media = template.resolve(media);
                            let timescale = segment_template.timescale.unwrap_or(1) as f32;

                            if let Some(x) = segment_template.duration {
                                segment_duration = x as f32 / timescale;
                            }

                            if segment_duration < 0.0 {
                                bail!(
                                    "Representation is missing SegmentTemplate@duration attribute."
                                );
                            }

                            let mut number = segment_template.startNumber.unwrap_or(1) as i64;
                            let total_number =
                                number + (period_duration_secs / segment_duration).round() as i64;

                            // // For a live manifest (dynamic MPD), we look at the time elapsed since now
                            // // and the mpd.availabilityStartTime to determine the correct value for
                            // // startNumber, based on duration and timescale. The latest available
                            // // segment is numbered
                            // //
                            // //    LSN = floor((now - (availabilityStartTime+PST))/segmentDuration + startNumber - 1)

                            // // https://dashif.org/Guidelines-TimingModel/Timing-Model.pdf
                            // // To be more precise, any LeapSecondInformation should be added to the availabilityStartTime.
                            // if mpd_is_dynamic(mpd) {
                            //     if let Some(start_time) = mpd.availabilityStartTime {
                            //         let elapsed = Utc::now()
                            //             .signed_duration_since(start_time)
                            //             .as_seconds_f64()
                            //             / segment_duration;
                            //         number = (elapsed + number as f64 - 1f64).floor() as u64;
                            //     } else {
                            //         return Err(DashMpdError::UnhandledMediaStream(
                            //             "dynamic manifest is missing @availabilityStartTime"
                            //                 .to_string(),
                            //         ));
                            //     }
                            // }

                            for _ in 1..=total_number {
                                template.insert("Number", number.to_string());

                                playlist.segments.push(Segment {
                                    duration: segment_duration,
                                    uri: base_url.join(&template.resolve(&media))?.to_string(),
                                    ..Default::default()
                                });

                                number += 1;
                            }
                        }
                    } else if let Some(segment_base) = &representation.SegmentBase {
                        // (5) SegmentBase@indexRange
                        if let Some(initialization) = &segment_base.Initialization {
                            let byte_range = parse_range(&initialization.range);

                            if let Some(source_url) = &initialization.sourceURL {
                                init_map = Some(Map {
                                    range: byte_range,
                                    uri: base_url.join(&template.resolve(source_url))?.to_string(),
                                });
                            } else {
                                init_map = Some(Map {
                                    range: byte_range,
                                    uri: base_url.to_string(),
                                });
                            }
                        }

                        if let Some(index_range) = parse_range(&segment_base.indexRange) {
                            let request = client
                                .get(base_url.as_str())
                                .query(query)
                                .header(header::RANGE, index_range.as_header_value());
                            let response = request.send().await?;
                            let bytes = response.bytes().await?;

                            if let Some(init_map) = &mut init_map {
                                init_map.range = Some(Range {
                                    end: index_range.end,
                                    start: 0,
                                })
                            }

                            for range in vsd_mp4::sidx::parse(&bytes, index_range.start)? {
                                playlist.segments.push(Segment {
                                    range: Some(Range {
                                        end: range.end,
                                        start: range.start,
                                    }),
                                    uri: base_url.to_string(),
                                    ..Default::default()
                                });
                            }
                        } else {
                            playlist.segments.push(Segment {
                                uri: base_url.to_string(),
                                ..Default::default()
                            });
                        }
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

                            // content_protection.value = "cenc" | "cbcs" | "cens" | "cbc1"
                            if encryption_type == KeyMethod::None
                                && content_protection.value.is_some()
                            {
                                encryption_type = KeyMethod::Mp4Decrypt;
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
                                    encryption_type = KeyMethod::Mp4Decrypt;
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
                                uri: None,
                            }),
                        };

                        first_segment.map = init_map;
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_frame_rate(frame_rate: &Option<String>) -> Option<f32> {
    frame_rate.as_ref().and_then(|frame_rate| {
        if frame_rate.contains('/') {
            if let Some((Some(upper), Some(lower))) = frame_rate
                .split_once('/')
                .map(|(x, y)| (x.parse::<f32>().ok(), y.parse::<f32>().ok()))
            {
                Some(upper / lower)
            } else {
                panic!("could'nt parse \"{frame_rate}\" frame rate");
            }
        } else {
            frame_rate.parse::<f32>().ok()
        }
    })
}

fn parse_range(range: &Option<String>) -> Option<Range> {
    range.as_ref().map(|range| {
        if let Some((Some(start), Some(end))) = range
            .split_once('-')
            .map(|(x, y)| (x.parse::<u64>().ok(), y.parse::<u64>().ok()))
        {
            Range { start, end }
        } else {
            panic!("could'nt parse \"{range}\" range");
        }
    })
}
