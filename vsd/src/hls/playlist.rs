use crate::playlist;

pub(crate) fn parse_as_master(
    m3u8: &m3u8_rs::MasterPlaylist,
    uri: &str,
) -> playlist::MasterPlaylist {
    let mut streams = vec![];

    for video_stream in &m3u8.variants {
        streams.push(playlist::MediaPlaylist {
            bandwidth: Some(video_stream.bandwidth),
            channels: None,
            codecs: video_stream.codecs.to_owned(),
            extension: Some("ts".to_owned()), // Cannot be comment here
            frame_rate: video_stream.frame_rate.map(|x| x as f32),
            i_frame: video_stream.is_i_frame,
            language: None,
            live: false, // Cannot be comment here
            media_type: playlist::MediaType::Video,
            playlist_type: playlist::PlaylistType::Hls,
            resolution: if let Some(m3u8_rs::Resolution { width, height }) = video_stream.resolution
            {
                Some((width, height))
            } else {
                None
            },
            segments: vec![], // Cannot be comment here
            uri: video_stream.uri.to_owned(),
        });
    }

    for alternative_stream in &m3u8.alternatives {
        if let Some(uri) = &alternative_stream.uri {
            match alternative_stream.media_type {
                m3u8_rs::AlternativeMediaType::Video => streams.push(playlist::MediaPlaylist {
                    bandwidth: None, // Cannot be comment here
                    channels: None,
                    codecs: None,                     // Cannot be comment here
                    extension: Some("ts".to_owned()), // Cannot be comment here
                    frame_rate: None,                 // Cannot be comment here
                    i_frame: false,                   // Cannot be comment here
                    language: None,
                    live: false, // Cannot be comment here
                    media_type: playlist::MediaType::Video,
                    playlist_type: playlist::PlaylistType::Hls,
                    resolution: None, // Cannot be comment here
                    segments: vec![], // Cannot be comment here
                    uri: uri.to_owned(),
                }),

                m3u8_rs::AlternativeMediaType::Audio => streams.push(playlist::MediaPlaylist {
                    bandwidth: None, // Cannot be comment here
                    channels: alternative_stream
                        .channels
                        .as_ref()
                        .map(|x| x.parse::<f32>().unwrap()),
                    codecs: None,                     // Cannot be comment here
                    extension: Some("ts".to_owned()), // Cannot be comment here
                    frame_rate: None,
                    i_frame: false,
                    language: alternative_stream
                        .language
                        .to_owned()
                        .or(alternative_stream.assoc_language.to_owned()),
                    live: false, // Cannot be comment here
                    media_type: playlist::MediaType::Audio,
                    playlist_type: playlist::PlaylistType::Hls,
                    resolution: None,
                    segments: vec![], // Cannot be comment here
                    uri: uri.to_owned(),
                }),

                m3u8_rs::AlternativeMediaType::ClosedCaptions
                | m3u8_rs::AlternativeMediaType::Subtitles => {
                    streams.push(playlist::MediaPlaylist {
                        bandwidth: None,
                        channels: None,
                        codecs: None,                      // Cannot be comment here
                        extension: Some("vtt".to_owned()), // Cannot be comment here
                        frame_rate: None,
                        i_frame: false,
                        language: alternative_stream
                            .language
                            .to_owned()
                            .or(alternative_stream.assoc_language.to_owned()),
                        live: false, // Cannot be comment here
                        media_type: playlist::MediaType::Subtitles,
                        playlist_type: playlist::PlaylistType::Hls,
                        resolution: None,
                        segments: vec![], // Cannot be comment here
                        uri: uri.to_owned(),
                    })
                }

                m3u8_rs::AlternativeMediaType::Other(_) => streams.push(playlist::MediaPlaylist {
                    bandwidth: None,
                    channels: alternative_stream
                        .channels
                        .as_ref()
                        .map(|x| x.parse::<f32>().unwrap()),
                    codecs: None,     // Cannot be comment here
                    extension: None,  // Cannot be comment here
                    frame_rate: None, // Cannot be comment here
                    i_frame: false,   // Cannot be comment here
                    language: alternative_stream
                        .language
                        .to_owned()
                        .or(alternative_stream.assoc_language.to_owned()),
                    live: false, // Cannot be comment here
                    media_type: playlist::MediaType::Undefined,
                    playlist_type: playlist::PlaylistType::Hls,
                    resolution: None, // Cannot be comment here
                    segments: vec![], // Cannot be comment here
                    uri: uri.to_owned(),
                }),
            }
        }
    }

    playlist::MasterPlaylist {
        playlist_type: playlist::PlaylistType::Hls,
        uri: uri.to_owned(),
        streams,
    }
}

pub(crate) fn push_segments(m3u8: &m3u8_rs::MediaPlaylist, playlist: &mut playlist::MediaPlaylist) {
    playlist.i_frame = m3u8.i_frames_only;
    playlist.live = !m3u8.end_list;

    let mut previous_byterange_end = 0;

    for segment in &m3u8.segments {
        let map = segment.map.as_ref().map(|x| playlist::Map {
            uri: x.uri.to_owned(),
            range: x.byte_range.as_ref().map(|x| {
                let offset = x.offset.unwrap_or(0);

                let (start, end) = if offset == 0 {
                    (
                        previous_byterange_end,
                        (previous_byterange_end + x.length) - 1,
                    )
                } else {
                    (x.length, (x.length + offset) - 1)
                };

                previous_byterange_end = end;
                playlist::Range { start, end }
            }),
        });

        let range = segment.byte_range.as_ref().map(|x| {
            let offset = x.offset.unwrap_or(0);

            let (start, end) = if offset == 0 {
                (
                    previous_byterange_end,
                    (previous_byterange_end + x.length) - 1,
                )
            } else {
                (x.length, (x.length + offset) - 1)
            };

            previous_byterange_end = end;
            playlist::Range { start, end }
        });

        playlist.segments.push(playlist::Segment {
            duration: segment.duration,
            key: if let Some(m3u8_rs::Key {
                iv,
                keyformat,
                method,
                uri,
                ..
            }) = &segment.key
            {
                let mut method = match method {
                    m3u8_rs::KeyMethod::AES128 => playlist::KeyMethod::Aes128,
                    m3u8_rs::KeyMethod::None => playlist::KeyMethod::None, // This should never match according to hls specifications.
                    m3u8_rs::KeyMethod::SampleAES => playlist::KeyMethod::SampleAes,
                    m3u8_rs::KeyMethod::Other(x)
                        if x == "SAMPLE-AES-CTR" || x == "SAMPLE-AES-CENC" =>
                    {
                        // cenc | cbc1 (pattern-based)
                        playlist::KeyMethod::Cenc
                    }
                    m3u8_rs::KeyMethod::Other(x) => playlist::KeyMethod::Other(x.to_owned()),
                };

                if let Some(keyformat) = keyformat {
                    method = match keyformat.as_str() {
                        "urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed"
                        | "com.apple.streamingkeydelivery"
                        | "com.microsoft.playready" => playlist::KeyMethod::Cenc, // cbcs (pattern-based) | cbc1
                        _ => method,
                    };
                }

                Some(playlist::Key {
                    default_kid: None,
                    iv: iv.clone(),
                    key_format: keyformat.clone(),
                    method,
                    uri: uri.clone(),
                })
            } else {
                None
            },
            map,
            range,
            uri: segment.uri.to_owned(),
        });
    }

    if let Some(segment) = playlist.segments.first() {
        if let Some(init) = &segment.map {
            if init.uri.split('?').next().unwrap().ends_with(".mp4") {
                playlist.extension = Some("m4s".to_owned());
            }
        }

        if let Some(extension) = segment
            .uri
            .split('?')
            .next()
            .unwrap()
            .split('/')
            .last()
            .and_then(|x| {
                if x.contains('.') {
                    x.split('.').last()
                } else {
                    Some("mp4")
                }
            })
        {
            playlist.extension = Some(extension.to_owned());
        }
    }
}
