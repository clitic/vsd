use crate::playlist;

pub(crate) fn parse_as_master(m3u8: &m3u8_rs::MasterPlaylist, uri: &str) -> playlist::MasterPlaylist {
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

    for segment in &m3u8.segments {
        playlist.segments.push(playlist::Segment {
            byte_range: segment.byte_range.as_ref().map(|x| playlist::ByteRange {
                length: x.length,
                offset: x.offset,
            }),
            duration: segment.duration,
            key: if let Some(m3u8_rs::Key {
                iv,
                keyformat,
                method,
                uri: Some(uri),
                ..
            }) = &segment.key
            {
                let method = match method {
                    m3u8_rs::KeyMethod::AES128 => playlist::KeyMethod::Aes128,
                    m3u8_rs::KeyMethod::None => playlist::KeyMethod::None, // This should never match according to hls specifications.
                    m3u8_rs::KeyMethod::SampleAES => playlist::KeyMethod::SampleAes,
                    // TODO - Match this with other queries and different key formats.
                    // Also check hls playlist examples where uri is not present but cenc is used.
                    m3u8_rs::KeyMethod::Other(x) if x.to_lowercase().contains("cenc") => {
                        playlist::KeyMethod::Cenc
                    }
                    m3u8_rs::KeyMethod::Other(x) => playlist::KeyMethod::Other(x.to_owned()),
                };

                Some(playlist::Key {
                    default_kid: None,
                    iv: iv.to_owned(),
                    key_format: keyformat.to_owned(),
                    method,
                    uri: uri.to_owned(),
                })
            } else {
                None
            },
            map: segment.map.as_ref().map(|x| playlist::Map {
                uri: x.uri.to_owned(),
                byte_range: x.byte_range.as_ref().map(|y| playlist::ByteRange {
                    length: y.length,
                    offset: y.offset,
                }),
            }),
            uri: segment.uri.to_owned(),
        });
    }

    if let Some(segment) = playlist.segments.get(0) {
        if let Some(init) = &segment.map {
            if init.uri.split('?').next().unwrap().ends_with(".mp4") {
                playlist.extension = Some("m4s".to_owned());
            }
        }

        if let Some(extension) = segment.uri.split('?').next().unwrap().split('.').last() {
            playlist.extension = Some(extension.to_owned());
        }
    }
}
