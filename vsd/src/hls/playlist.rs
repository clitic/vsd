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
            id: String::new(), // Cannot be comment here
            i_frame: video_stream.is_i_frame,
            language: None,
            live: false, // Cannot be comment here
            media_sequence: 0,
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
                    id: String::new(),                // Cannot be comment here
                    i_frame: false,                   // Cannot be comment here
                    language: None,
                    live: false, // Cannot be comment here
                    media_sequence: 0,
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
                    id: String::new(), // Cannot be comment here
                    i_frame: false,
                    language: alternative_stream
                        .language
                        .to_owned()
                        .or(alternative_stream.assoc_language.to_owned()),
                    live: false, // Cannot be comment here
                    media_sequence: 0,
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
                        id: String::new(), // Cannot be comment here
                        i_frame: false,
                        language: alternative_stream
                            .language
                            .to_owned()
                            .or(alternative_stream.assoc_language.to_owned()),
                        live: false, // Cannot be comment here
                        media_sequence: 0,
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
                    codecs: None,      // Cannot be comment here
                    extension: None,   // Cannot be comment here
                    frame_rate: None,  // Cannot be comment here
                    id: String::new(), // Cannot be comment here
                    i_frame: false,    // Cannot be comment here
                    language: alternative_stream
                        .language
                        .to_owned()
                        .or(alternative_stream.assoc_language.to_owned()),
                    live: false, // Cannot be comment here
                    media_sequence: 0,
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

pub(crate) fn push_segments(m3u8: &m3u8_rs::MediaPlaylist, stream: &mut playlist::MediaPlaylist) {
    stream.i_frame = m3u8.i_frames_only;
    stream.live = !m3u8.end_list;
    stream.media_sequence = m3u8.media_sequence;

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

        stream.segments.push(playlist::Segment {
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
                        if x == "SAMPLE-AES-CENC" || x == "SAMPLE-AES-CTR" =>
                    {
                        playlist::KeyMethod::Mp4Decrypt
                    }
                    m3u8_rs::KeyMethod::Other(x) => playlist::KeyMethod::Other(x.to_owned()),
                };

                /*
                    .mpd (with encryption) converted to .m3u8

                    #EXT-X-KEY:METHOD=SAMPLE-AES,URI="skd://302f80dd-411e-4886-bca5-bb1f8018a024:77FD1889AAF4143B085548B3C0F95B9A",KEYFORMATVERSIONS="1",KEYFORMAT="com.apple.streamingkeydelivery"
                    #EXT-X-KEY:METHOD=SAMPLE-AES-CTR,KEYFORMAT="com.microsoft.playready",KEYFORMATVERSIONS="1",URI="data:text/plain;charset=UTF-16;base64,xAEAAAEAAQC6ATwAVwBSAE0ASABFAEEARABFAFIAIAB4AG0AbABuAHMAPQAiAGgAdAB0AHAAOgAvAC8AcwBjAGgAZQBtAGEAcwAuAG0AaQBjAHIAbwBzAG8AZgB0AC4AYwBvAG0ALwBEAFIATQAvADIAMAAwADcALwAwADMALwBQAGwAYQB5AFIAZQBhAGQAeQBIAGUAYQBkAGUAcgAiACAAdgBlAHIAcwBpAG8AbgA9ACIANAAuADAALgAwAC4AMAAiAD4APABEAEEAVABBAD4APABQAFIATwBUAEUAQwBUAEkATgBGAE8APgA8AEsARQBZAEwARQBOAD4AMQA2ADwALwBLAEUAWQBMAEUATgA+ADwAQQBMAEcASQBEAD4AQQBFAFMAQwBUAFIAPAAvAEEATABHAEkARAA+ADwALwBQAFIATwBUAEUAQwBUAEkATgBGAE8APgA8AEsASQBEAD4AOQBmAEIAMQAxAEsAMQB0AC8ARQBtAFEANABYAEMATQBjAEoANgBnAEkAZwA9AD0APAAvAEsASQBEAD4APAAvAEQAQQBUAEEAPgA8AC8AVwBSAE0ASABFAEEARABFAFIAPgA="
                    #EXT-X-KEY:METHOD=SAMPLE-AES,URI="data:text/plain;base64,AAAAXHBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAADwSEDAvgN1BHkiGvKW7H4AYoCQSEDAvgN1BHkiGvKW7H4AYoCQSEDAvgN1BHkiGvKW7H4AYoCRI88aJmwY=",KEYID=0x302F80DD411E4886BCA5BB1F8018A024,IV=0x77FD1889AAF4143B085548B3C0F95B9A,KEYFORMATVERSIONS="1",KEYFORMAT="urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed"

                    https://dashif.org/identifiers/content_protection
                */
                if let Some(keyformat) = keyformat {
                    method = match keyformat.as_str() {
                        "com.apple.streamingkeydelivery"
                        | "com.microsoft.playready"
                        | "urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed" => {
                            playlist::KeyMethod::Mp4Decrypt
                        }
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

    if let Some(segment) = stream.segments.first() {
        if let Some(init) = &segment.map {
            if init.uri.split('?').next().unwrap().ends_with(".mp4") {
                stream.extension = Some("m4s".to_owned());
            }
        }

        let uri = segment.uri.split('?').next().unwrap();

        if uri.ends_with(".mp4") || uri.ends_with(".m4s") {
            stream.extension = Some("m4s".to_owned());
        }
    }
}
