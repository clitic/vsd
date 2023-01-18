// REFERENCES: https://github.com/nilaoda/N_m3u8DL-RE/blob/main/src/N_m3u8DL-RE.Parser/Extractor/DASHExtractor2.cs

use super::utils;
use super::{AdaptationSet, PlaylistTag, Representation, SegmentTag, TemplateResolver, MPD};
use crate::playlist;
use anyhow::{anyhow, bail, Result};

pub fn as_master(mpd: &MPD, uri: &str) -> playlist::MasterPlaylist {
    let mut variants = vec![];

    for (period_index, period) in mpd.period.iter().enumerate() {
        for (adaptation_set_index, adaptation_set) in period.adaptation_set.iter().enumerate() {
            for (representation_index, representation) in
                adaptation_set.representation.iter().enumerate()
            {
                // TODO: Add these fields
                // representation.codecs(adaptation_set)
                // representation.frame_rate(adaptation_set)
                // representation.extension(adaptation_set)

                variants.push(playlist::MediaPlaylist {
                    bandwidth: representation.bandwidth,
                    channels: representation.channels(adaptation_set),
                    init_segment: None,
                    language: representation.lang(adaptation_set),
                    media_type: representation.media_type(adaptation_set),
                    playlist_type: playlist::PlaylistType::Dash,
                    resolution: if let (Some(width), Some(height)) =
                        (representation.width, representation.height)
                    {
                        Some((width, height))
                    } else {
                        None
                    },
                    segments: vec![],
                    uri: format!(
                        "dash://period.{}.adaptation-set.{}.representation.{}",
                        period_index, adaptation_set_index, representation_index
                    ),
                });
            }
        }
    }

    playlist::MasterPlaylist {
        playlist_type: playlist::PlaylistType::Dash,
        uri: uri.to_owned(),
        variants,
    }
}

pub fn as_media(mpd: &MPD, dash_uri: &str, baseurl: &str) -> Result<playlist::MediaPlaylist> {
    if !dash_uri.starts_with("dash://") {
        bail!(
            "incorrect MPD uri format (expected: dash://period.{{}}.adaptation-set.{{}}.representation.{{}})"
        )
    }

    let location = dash_uri
        .split('.')
        .filter_map(|x| x.parse::<usize>().ok())
        .collect::<Vec<usize>>();

    if location.len() != 3 {
        bail!(
            "incorrect MPD uri format (expected: dash://period.{{}}.adaptation-set.{{}}.representation.{{}})"
        )
    }

    let period = &mpd
        .period
        .get(location[0])
        .ok_or_else(|| anyhow!("requested MPD playlist not found"))?;
    let adaptation_set = &period
        .adaptation_set
        .get(location[1])
        .ok_or_else(|| anyhow!("requested MPD playlist not found"))?;
    let representation = &adaptation_set
        .representation
        .get(location[2])
        .ok_or_else(|| anyhow!("requested MPD playlist not found"))?;

    // BASEURL
    let mut baseurl = baseurl.parse::<reqwest::Url>()?;

    if let Some(mpd_baseurl) = &mpd.baseurl {
        baseurl = baseurl.join(mpd_baseurl)?;
    }

    if let Some(period_baseurl) = &period.baseurl {
        baseurl = baseurl.join(period_baseurl)?;
    }

    if let Some(adaptation_set_baseurl) = &adaptation_set.baseurl {
        baseurl = baseurl.join(adaptation_set_baseurl)?;
    }

    if let Some(representation_baseurl) = &representation.baseurl {
        baseurl = baseurl.join(representation_baseurl)?;
    }

    // MPD DURATION
    let mpd_duration = period.duration(mpd);

    // KEYS
    let key = match representation.encryption_type(adaptation_set) {
        playlist::KeyMethod::None => None,
        x => Some(playlist::Key {
            default_kid: representation
                .default_kid(adaptation_set)
                .map(|x| x.replace('-', "").to_lowercase()),
            iv: None,
            method: x,
            uri: "dash://encryption-key".to_owned(),
        }),
    };

    // SEGMENTS
    let mut init_segment = None;
    let mut segments = vec![];

    if let Some(segment_base) = &representation.segment_base {
        if let Some(initialization) = &segment_base.initialization {
            if let Some(source_url) = &initialization.source_url {
                init_segment = Some(playlist::Segment {
                    byte_range: mpd_range_to_byte_range(&initialization.range),
                    uri: baseurl.join(source_url)?.as_str().to_owned(),
                    ..Default::default()
                });
            } else {
                init_segment = Some(playlist::Segment {
                    duration: mpd_duration,
                    uri: baseurl.as_str().to_owned(),
                    ..Default::default()
                });
            }
        }
    }

    if let Some(segment_list) = &representation.segment_list {
        if let Some(initialization) = &segment_list.initialization {
            if let Some(source_url) = &initialization.source_url {
                init_segment = Some(playlist::Segment {
                    byte_range: mpd_range_to_byte_range(&initialization.range),
                    uri: baseurl.join(source_url)?.as_str().to_owned(),
                    ..Default::default()
                });
            }
        }

        let duration = segment_list.segment_duration();

        for segment_url in &segment_list.segment_urls {
            segments.push(playlist::Segment {
                byte_range: mpd_range_to_byte_range(&segment_url.media_range),
                duration,
                key, // clone
                map: None,
                uri: baseurl.join(segment_url.media.as_ref().unwrap())?.as_str().to_owned(),
            });
        }
    }

    if let Some(segment_template) = representation.segment_template(adaptation_set) {
        let mut template_resolver = TemplateResolver::new(representation.template_vars());

        if let Some(initialization) = &segment_template.initialization {
            init_segment = Some(m3u8_rs::MediaSegment {
                uri: template_resolver.resolve(&utils::join_url(&baseurl, initialization)?),
                unknown_tags: SegmentTag::default().init(true).build().into(),
                ..Default::default()
            });

            let mut start_number = segment_template.start_number();
            let timescale = segment_template.timescale();

            if let Some(segment_timeline) = &segment_template.segment_timeline {
                let mut current_time = 0;

                for s in &segment_timeline.s {
                    if let Some(t) = &s.t {
                        current_time = *t;
                    }

                    template_resolver.insert("Time", current_time.to_string());
                    template_resolver.insert("Number", start_number.to_string());

                    segments.push(m3u8_rs::MediaSegment {
                        uri: template_resolver.resolve(&utils::join_url(
                            &baseurl,
                            segment_template.media.as_ref().unwrap(),
                        )?),
                        duration: s.d as f32 / timescale,
                        key: key.clone(),
                        unknown_tags: unknown_tags.clone(),
                        ..Default::default()
                    });

                    start_number += 1;

                    let mut repeat_count = s.r.unwrap_or(0);

                    if repeat_count < 0 {
                        repeat_count = ((mpd_duration * timescale / s.d as f32) - 1.0) as i64;
                    }

                    for _ in 0..repeat_count {
                        current_time += s.d;

                        template_resolver.insert("Time", current_time.to_string());
                        template_resolver.insert("Number", start_number.to_string());

                        segments.push(m3u8_rs::MediaSegment {
                            uri: template_resolver.resolve(&utils::join_url(
                                &baseurl,
                                segment_template.media.as_ref().unwrap(),
                            )?),
                            duration: s.d as f32 / timescale,
                            key: key.clone(),
                            unknown_tags: unknown_tags.clone(),
                            ..Default::default()
                        });

                        start_number += 1;
                    }

                    current_time += s.d;
                }
            } else {
                let duration = segment_template.duration();
                let segment_duration = duration / timescale;
                let mut total = (mpd_duration * timescale / duration).ceil() as usize;

                if total == 0 && mpd.live() {
                    let now = if let Some(publish_time) = &mpd.publish_time {
                        chrono::DateTime::parse_from_rfc3339(publish_time).unwrap()
                    } else {
                        chrono::Local::now().into()
                    };

                    let available_time = chrono::DateTime::parse_from_rfc3339(
                        mpd.availability_start_time.as_ref().unwrap(),
                    )
                    .unwrap();
                    let ts = now - available_time;
                    let update_ts = utils::iso8601_duration_to_seconds(
                        mpd.time_shift_buffer_depth.as_ref().unwrap(),
                    )
                    .unwrap();
                    start_number +=
                        ((ts.num_seconds() as f32 - update_ts) * timescale / duration) as usize;
                    total = (update_ts * timescale / duration) as usize;
                }

                for i in start_number..(start_number + total) {
                    template_resolver.insert("Number", i.to_string());

                    segments.push(m3u8_rs::MediaSegment {
                        uri: template_resolver.resolve(&utils::join_url(
                            &baseurl,
                            segment_template.media.as_ref().unwrap(),
                        )?),
                        duration: segment_duration,
                        key: key.clone(),
                        unknown_tags: unknown_tags.clone(),
                        ..Default::default()
                    });
                }
            }
        }
    }

    if segments.is_empty() {
        segments.push(m3u8_rs::MediaSegment {
            uri: baseurl,
            duration: mpd_duration,
            key,
            unknown_tags: SegmentTag::default()
                .kid(representation.default_kid(adaptation_set))
                .single(true)
                .build()
                .into(),
            ..Default::default()
        });
    }

    if let Some(init_segment) = init_segment {
        segments.insert(0, init_segment);
    }

    Ok(m3u8_rs::MediaPlaylist {
        segments,
        end_list: true,
        unknown_tags: PlaylistTag::default()
            .codecs(representation.codecs(adaptation_set))
            .bandwidth(representation.bandwidth.map(|x| x as usize))
            .extension(representation.extension(adaptation_set))
            .build()
            .into(),
        ..Default::default()
    })
}

fn mpd_range_to_byte_range(range: &Option<String>) -> Option<playlist::ByteRange> {
    range.as_ref().map(|range| playlist::ByteRange {
        length: range.split('-').next().unwrap().parse::<u64>().unwrap(),
        offset: Some(range.split('-').nth(1).unwrap().parse::<u64>().unwrap()),
    })
}
