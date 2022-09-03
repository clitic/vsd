use super::utils;
use super::{AdaptationSet, MPDMediaSegmentTag, Representation, MPD};

pub fn to_m3u8_as_master(mpd: &MPD) -> m3u8_rs::MasterPlaylist {
    let mut master = m3u8_rs::MasterPlaylist::default();

    for (period_index, period) in mpd.period.iter().enumerate() {
        for (adaptation_set_index, adaptation_set) in period.adaptation_set.iter().enumerate() {
            for (representation_index, representation) in
                adaptation_set.representation.iter().enumerate()
            {
                let media_type = representation.media_type(&adaptation_set);

                if media_type == m3u8_rs::AlternativeMediaType::Video {
                    master.variants.push(m3u8_rs::VariantStream {
                        uri: format!(
                            "dash://period.{}.adaptation-set.{}.representation.{}",
                            period_index, adaptation_set_index, representation_index
                        ),
                        bandwidth: representation.bandwidth.clone().unwrap(),
                        codecs: representation.codecs(&adaptation_set),
                        resolution: if let (Some(width), Some(height)) =
                            (representation.width, representation.height)
                        {
                            Some(m3u8_rs::Resolution { width, height })
                        } else {
                            None
                        },
                        frame_rate: representation.frame_rate(&adaptation_set),
                        ..Default::default()
                    });
                } else {
                    master.alternatives.push(m3u8_rs::AlternativeMedia {
                        media_type,
                        uri: Some(format!(
                            "dash://period.{}.adaptation-set.{}.representation.{}",
                            period_index, adaptation_set_index, representation_index
                        )),
                        language: representation.lang(&adaptation_set),
                        assoc_language: representation.lang(&adaptation_set),
                        channels: representation.channels(&adaptation_set),
                        characteristics: if let Some(bandwidth) = representation.bandwidth {
                            Some(format!("BANDWIDTH={}", bandwidth))
                        } else {
                            None
                        },
                        ..Default::default()
                    });
                }
            }
        }
    }

    let mut alternative_audio = vec![];
    let mut alternative_subtitles = vec![];

    for (i, alternative) in master.alternatives.iter().enumerate() {
        let bandwidth = alternative
            .characteristics
            .clone()
            .unwrap_or("BANDWIDTH=0".to_owned())
            .split('=')
            .nth(1)
            .unwrap()
            .parse::<usize>()
            .unwrap();

        if alternative.media_type == m3u8_rs::AlternativeMediaType::Audio {
            if alternative.uri.is_some() {
                alternative_audio.push((i, bandwidth));
            }
        }

        if alternative.media_type == m3u8_rs::AlternativeMediaType::Subtitles {
            if alternative.uri.is_some() {
                alternative_subtitles.push((i, bandwidth));
            }
        }
    }

    if alternative_audio.len() > 1 {
        alternative_audio.sort_by(|x, y| y.1.cmp(&x.1));
    }

    if alternative_subtitles.len() > 1 {
        alternative_subtitles.sort_by(|x, y| y.1.cmp(&x.1));
    }

    if alternative_audio.len() != 0 {
        master
            .alternatives
            .get_mut(alternative_audio[0].0)
            .unwrap()
            .autoselect = true;
    }

    if alternative_subtitles.len() != 0 {
        master
            .alternatives
            .get_mut(alternative_subtitles[0].0)
            .unwrap()
            .autoselect = true;
    }

    master
}

pub fn to_m3u8_as_media(mpd: &MPD, mpd_url: &str, uri: &str) -> Option<m3u8_rs::MediaPlaylist> {
    for (period_index, period) in mpd.period.iter().enumerate() {
        let mut baseurl = mpd_url.clone().to_owned();

        if let Some(period_baseurl) = &period.baseurl {
            baseurl = utils::join_url(&baseurl, &period_baseurl).unwrap();
        }

        for (adaptation_set_index, adaptation_set) in period.adaptation_set.iter().enumerate() {
            if let Some(adaptation_set_baseurl) = &adaptation_set.baseurl {
                baseurl = utils::join_url(&baseurl, &adaptation_set_baseurl).unwrap();
            }

            for (representation_index, representation) in
                adaptation_set.representation.iter().enumerate()
            {
                if format!(
                    "dash://period.{}.adaptation-set.{}.representation.{}",
                    period_index, adaptation_set_index, representation_index
                ) != uri
                {
                    continue;
                }

                if let Some(representation_baseurl) = &representation.baseurl {
                    baseurl = utils::join_url(&baseurl, &representation_baseurl).unwrap();
                }

                let mut init_segment = None;
                let mut segments = vec![];

                if let Some(segment_base) = &representation.segment_base {
                    if let Some(initialization) = &segment_base.initialization {
                        if let Some(source_url) = &initialization.source_url {
                            init_segment = Some(m3u8_rs::MediaSegment {
                                uri: utils::join_url(&baseurl, &source_url).unwrap(),
                                byte_range: if let Some((start, end)) = &initialization.range() {
                                    Some(m3u8_rs::ByteRange {
                                        length: *start,
                                        offset: Some(*end),
                                    })
                                } else {
                                    None
                                },
                                unknown_tags: MPDMediaSegmentTag::default()
                                    .init(true)
                                    .build()
                                    .into(),
                                ..Default::default()
                            });
                        } else {
                            init_segment = Some(m3u8_rs::MediaSegment {
                                uri: baseurl.clone(),
                                duration: period.duration(&mpd).unwrap(),
                                unknown_tags: MPDMediaSegmentTag::default()
                                    .init(true)
                                    .build()
                                    .into(),
                                ..Default::default()
                            });
                        }
                    }
                }

                if let Some(segment_list) = &representation.segment_list {
                    if let Some(initialization) = &segment_list.initialization {
                        if let Some(source_url) = &initialization.source_url {
                            init_segment = Some(m3u8_rs::MediaSegment {
                                uri: utils::join_url(&baseurl, &source_url).unwrap(),
                                byte_range: if let Some((start, end)) = &initialization.range() {
                                    Some(m3u8_rs::ByteRange {
                                        length: *start,
                                        offset: Some(*end),
                                    })
                                } else {
                                    None
                                },
                                unknown_tags: MPDMediaSegmentTag::default()
                                    .init(true)
                                    .build()
                                    .into(),
                                ..Default::default()
                            });
                        }
                    }

                    let timescale = segment_list
                        .timescale
                        .clone()
                        .unwrap_or("1".to_owned())
                        .parse::<f32>()
                        .unwrap();

                    for segment_url in &segment_list.segment_urls {
                        segments.push(m3u8_rs::MediaSegment {
                            uri: utils::join_url(&baseurl, &segment_url.media.clone().unwrap())
                                .unwrap(),
                            duration: segment_list
                                .duration
                                .clone()
                                .unwrap()
                                .parse::<f32>()
                                .unwrap()
                                / timescale,
                            byte_range: if let Some((start, end)) = &segment_url.range() {
                                Some(m3u8_rs::ByteRange {
                                    length: *start,
                                    offset: Some(*end),
                                })
                            } else {
                                None
                            },
                            key: mpd_to_m3u8_key(&representation, &adaptation_set),
                            unknown_tags: MPDMediaSegmentTag::default()
                                .kid(representation.default_kid(&adaptation_set))
                                .into(),
                            ..Default::default()
                        });
                    }
                }

                let segment_template =
                    if let Some(segment_template) = &adaptation_set.segment_template {
                        Some(segment_template)
                    } else if let Some(segment_template) = &representation.segment_template {
                        Some(segment_template)
                    } else {
                        None
                    };

                if let Some(segment_template) = segment_template {
                    if let Some(initialization) = &segment_template.initialization {
                        init_segment = Some(m3u8_rs::MediaSegment {
                            uri: utils::resolve_url_template(
                                &utils::join_url(&baseurl, initialization).unwrap(),
                                &representation.template_vars(),
                            ),
                            unknown_tags: MPDMediaSegmentTag::default().init(true).build().into(),
                            ..Default::default()
                        });

                        let mut start_number = segment_template.start_number.clone().unwrap_or(0);
                        let timescale = segment_template
                            .timescale
                            .clone()
                            .unwrap()
                            .parse::<f32>()
                            .unwrap();

                        if let Some(segment_timeline) = &segment_template.segment_timeline {
                            let mut current_time = 0;

                            for s in &segment_timeline.s {
                                if let Some(t) = &s.t {
                                    current_time = *t;
                                }

                                let mut template_vars = representation.template_vars();
                                template_vars.insert("Time", current_time.to_string());
                                template_vars.insert("Number", start_number.to_string());

                                segments.push(m3u8_rs::MediaSegment {
                                    uri: utils::resolve_url_template(
                                        &utils::join_url(
                                            &baseurl,
                                            &segment_template.media.clone().unwrap(),
                                        )
                                        .unwrap(),
                                        &template_vars,
                                    ),
                                    duration: s.d as f32 / timescale,
                                    key: mpd_to_m3u8_key(&representation, &adaptation_set),
                                    unknown_tags: MPDMediaSegmentTag::default()
                                        .kid(representation.default_kid(&adaptation_set))
                                        .into(),
                                    ..Default::default()
                                });

                                start_number += 1;

                                let mut repeat_count = s.r.unwrap();

                                if repeat_count < 0 {
                                    let duration = if let Some(duration) = &period.duration {
                                        utils::iso8601_duration_to_seconds(&duration).unwrap()
                                    } else if let Some(duration) = &mpd.media_presentation_duration
                                    {
                                        utils::iso8601_duration_to_seconds(&duration).unwrap()
                                    } else {
                                        0.0
                                    };

                                    repeat_count =
                                        ((duration * timescale / s.d as f32) - 1.0) as i64;
                                }

                                for _ in 0..repeat_count {
                                    current_time += s.d;

                                    let mut template_vars = representation.template_vars();
                                    template_vars.insert("Time", current_time.to_string());
                                    template_vars.insert("Number", start_number.to_string());

                                    segments.push(m3u8_rs::MediaSegment {
                                        uri: utils::resolve_url_template(
                                            &utils::join_url(
                                                &baseurl,
                                                &segment_template.media.clone().unwrap(),
                                            )
                                            .unwrap(),
                                            &template_vars,
                                        ),
                                        duration: s.d as f32 / timescale,
                                        key: mpd_to_m3u8_key(&representation, &adaptation_set),
                                        unknown_tags: MPDMediaSegmentTag::default()
                                            .kid(representation.default_kid(&adaptation_set))
                                            .into(),
                                        ..Default::default()
                                    });

                                    start_number += 1;
                                }

                                current_time += s.d;
                            }
                        } else {
                            let duration = if let Some(duration) = &period.duration {
                                utils::iso8601_duration_to_seconds(&duration).unwrap()
                            } else if let Some(duration) = &mpd.media_presentation_duration {
                                utils::iso8601_duration_to_seconds(&duration).unwrap()
                            } else {
                                0.0
                            };

                            let total = (duration * timescale
                                / segment_template
                                    .duration
                                    .clone()
                                    .unwrap()
                                    .parse::<f32>()
                                    .unwrap())
                            .ceil() as usize;

                            if total == 0 && mpd.live() {
                                // let now = if let Some(publish_time) = &mpd.publish_time {
                                //     iso8601::datetime(&publish_time).unwrap()
                                // } else {
                                //     // iso8601::DateTime::from()
                                // };

                                // mpd.availability_start_time.clone().unwrap();

                                // var now = publishTime == null ? DateTime.Now : DateTime.Parse(publishTime);
                                // var availableTime = DateTime.Parse(availabilityStartTime!);
                                // var ts = now - availableTime;
                                // var updateTs = XmlConvert.ToTimeSpan(timeShiftBufferDepth!);
                                // startNumber += (long)((ts.TotalSeconds - updateTs.TotalSeconds) * timescale / duration);
                                // totalNumber = (long)(updateTs.TotalSeconds * timescale / duration);
                                todo!("TODO SUPPORT LIVE STREAMS")
                            }

                            for _ in start_number..(start_number + total) {
                                // {
                                //     varDic[DASHTags.TemplateNumber] = index;
                                //     var mediaUrl = ParserUtil.ReplaceVars(ParserUtil.CombineURL(segBaseUrl, media!), varDic);
                                //     MediaSegment mediaSegment = new();
                                //     mediaSegment.Url = mediaUrl;
                                //     mediaSegment.Index = isLive ? index : segIndex; //直播直接用startNumber
                                //     mediaSegment.Duration = duration / (double)timescale;
                                //     streamSpec.Playlist.MediaParts[0].MediaSegments.Add(mediaSegment);
                                // }

                                let mut template_vars = representation.template_vars();
                                template_vars.insert("Number", start_number.to_string());

                                segments.push(m3u8_rs::MediaSegment {
                                    uri: utils::resolve_url_template(
                                        &utils::join_url(
                                            &baseurl,
                                            &segment_template.media.clone().unwrap(),
                                        )
                                        .unwrap(),
                                        &template_vars,
                                    ),
                                    duration: segment_template
                                        .duration
                                        .clone()
                                        .unwrap()
                                        .parse::<f32>()
                                        .unwrap()
                                        / timescale,
                                    key: mpd_to_m3u8_key(&representation, &adaptation_set),
                                    unknown_tags: MPDMediaSegmentTag::default()
                                        .kid(representation.default_kid(&adaptation_set))
                                        .into(),
                                    ..Default::default()
                                });

                                start_number += 1;
                            }
                        }
                    }
                }

                if segments.len() == 0 {
                    segments.push(m3u8_rs::MediaSegment {
                        uri: baseurl.clone(),
                        duration: period.duration(&mpd).unwrap(),
                        key: mpd_to_m3u8_key(&representation, &adaptation_set),
                        unknown_tags: MPDMediaSegmentTag::default()
                            .kid(representation.default_kid(&adaptation_set))
                            .single(true)
                            .build()
                            .into(),
                        ..Default::default()
                    });
                }

                if let Some(init_segment) = init_segment {
                    segments.insert(0, init_segment);
                }

                return Some(m3u8_rs::MediaPlaylist {
                    segments,
                    end_list: true,
                    ..Default::default()
                });
            }
        }
    }

    None
}

fn mpd_to_m3u8_key(
    representation: &Representation,
    adaptation_set: &AdaptationSet,
) -> Option<m3u8_rs::Key> {
    if let Some(key) = representation.encryption_type(&adaptation_set) {
        Some(m3u8_rs::Key {
            method: m3u8_rs::KeyMethod::Other(key),
            ..Default::default()
        })
    } else {
        None
    }
}
