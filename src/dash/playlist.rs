/*
    REFERENCES
    ----------

    1. https://github.com/nilaoda/N_m3u8DL-RE/blob/7bba10aa0d7adf7e79e0feec7327039681cb7bd4/src/N_m3u8DL-RE.Parser/Extractor/DASHExtractor2.cs

*/

use super::{iso8601_duration_to_seconds, mpd_range_to_byte_range, DashUrl, TemplateResolver, MPD};
use crate::playlist;
use anyhow::{anyhow, Result};

pub fn parse_as_master(mpd: &MPD, uri: &str) -> playlist::MasterPlaylist {
    let mut streams = vec![];

    for (period_index, period) in mpd.period.iter().enumerate() {
        for (adaptation_set_index, adaptation_set) in period.adaptation_set.iter().enumerate() {
            for (representation_index, representation) in
                adaptation_set.representation.iter().enumerate()
            {
                streams.push(playlist::MediaPlaylist {
                    bandwidth: representation.bandwidth,
                    channels: representation.channels(adaptation_set),
                    codecs: representation.codecs(adaptation_set),
                    extension: representation.extension(adaptation_set),
                    frame_rate: representation.frame_rate(adaptation_set),
                    i_frame: false, // cannot comment here
                    language: representation.lang(adaptation_set),
                    live: mpd.live(),
                    media_type: representation.media_type(adaptation_set),
                    playlist_type: playlist::PlaylistType::Dash,
                    resolution: if let (Some(width), Some(height)) =
                        (representation.width, representation.height)
                    {
                        Some((width, height))
                    } else {
                        None
                    },
                    segments: vec![], // cannot comment here
                    uri: DashUrl::new(period_index, adaptation_set_index, representation_index)
                        .to_string(),
                });
            }
        }
    }

    playlist::MasterPlaylist {
        playlist_type: playlist::PlaylistType::Dash,
        uri: uri.to_owned(),
        streams,
    }
}

pub fn push_segments(
    mpd: &MPD,
    playlist: &mut playlist::MediaPlaylist,
    baseurl: &str,
) -> Result<()> {
    let location = playlist.uri.parse::<DashUrl>().map_err(|x| anyhow!(x))?;

    let period = mpd
        .period
        .get(location.period)
        .ok_or_else(|| anyhow!("dash period.{} could't be located", location.period))?;

    let adaptation_set = &period
        .adaptation_set
        .get(location.adaptation_set)
        .ok_or_else(|| {
            anyhow!(
                "dash adaptation-set.{} could't be located",
                location.adaptation_set
            )
        })?;

    let representation = &adaptation_set
        .representation
        .get(location.representation)
        .ok_or_else(|| {
            anyhow!(
                "dash representation.{} could't be located",
                location.representation
            )
        })?;

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

    // SEGMENTS
    let mut init_map = None;

    if let Some(segment_base) = &representation.segment_base {
        if let Some(initialization) = &segment_base.initialization {
            if let Some(source_url) = &initialization.source_url {
                init_map = Some(playlist::Map {
                    byte_range: mpd_range_to_byte_range(&initialization.range),
                    uri: baseurl.join(source_url)?.as_str().to_owned(),
                });
            } else {
                init_map = Some(playlist::Map {
                    byte_range: None,
                    // duration: mpd_duration,
                    uri: baseurl.as_str().to_owned(),
                });
            }
        }
    }

    if let Some(segment_list) = &representation.segment_list {
        if let Some(initialization) = &segment_list.initialization {
            if let Some(source_url) = &initialization.source_url {
                init_map = Some(playlist::Map {
                    byte_range: mpd_range_to_byte_range(&initialization.range),
                    uri: baseurl.join(source_url)?.as_str().to_owned(),
                });
            }
        }

        let duration = segment_list.segment_duration();

        for segment_url in &segment_list.segment_urls {
            playlist.segments.push(playlist::Segment {
                byte_range: mpd_range_to_byte_range(&segment_url.media_range),
                duration,
                uri: baseurl
                    .join(segment_url.media.as_ref().unwrap())?
                    .as_str()
                    .to_owned(),
                ..Default::default()
            });
        }
    }

    if let Some(segment_template) = representation.segment_template(adaptation_set) {
        let mut template_resolver = TemplateResolver::new(representation.template_vars());

        if let Some(initialization) = &segment_template.initialization {
            init_map = Some(playlist::Map {
                byte_range: None,
                uri: template_resolver.resolve(baseurl.join(initialization)?.as_str()),
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

                    playlist.segments.push(playlist::Segment {
                        duration: s.d as f32 / timescale,
                        uri: template_resolver.resolve(
                            baseurl
                                .join(segment_template.media.as_ref().unwrap())?
                                .as_str(),
                        ),
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

                        playlist.segments.push(playlist::Segment {
                            duration: s.d as f32 / timescale,
                            uri: template_resolver.resolve(
                                baseurl
                                    .join(segment_template.media.as_ref().unwrap())?
                                    .as_str(),
                            ),
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
                    let update_ts =
                        iso8601_duration_to_seconds(mpd.time_shift_buffer_depth.as_ref().unwrap())
                            .unwrap();
                    start_number +=
                        ((ts.num_seconds() as f32 - update_ts) * timescale / duration) as usize;
                    total = (update_ts * timescale / duration) as usize;
                }

                for i in start_number..(start_number + total) {
                    template_resolver.insert("Number", i.to_string());

                    playlist.segments.push(playlist::Segment {
                        duration: segment_duration,
                        uri: template_resolver.resolve(
                            baseurl
                                .join(segment_template.media.as_ref().unwrap())?
                                .as_str(),
                        ),
                        ..Default::default()
                    });
                }
            }
        }
    }

    if playlist.segments.is_empty() {
        // single
        playlist.segments.push(playlist::Segment {
            duration: mpd_duration,
            uri: baseurl.as_str().to_owned(),
            ..Default::default()
        });
    }

    if let Some(first_segment) = playlist.segments.get_mut(0) {
        first_segment.key = match representation.encryption_type(adaptation_set) {
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

        first_segment.map = init_map;
    }

    Ok(())
}
