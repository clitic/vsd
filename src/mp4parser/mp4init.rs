/*
    REFERENCES
    ----------

    1. https://github.com/nilaoda/N_m3u8DL-RE/blob/f2976d3f92b03ee5759375394239e6d423198ce1/src/N_m3u8DL-RE.Parser/Mp4/MP4InitUtil.cs

*/

use crate::mp4parser;
use crate::mp4parser::MP4Parser;
use std::sync::Arc;

pub(crate) struct MP4Init {
    kid: Option<String>,
    pssh: Option<String>,
    scheme: Option<String>,
}

impl MP4Init {
    pub(crate) fn new(data: &[u8]) -> Result<Self, String> {
        let system_id_widevine: Vec<u8> = vec![
            237, 239, 139, 169, 121, 214, 74, 206, 163, 200, 39, 220, 213, 29, 33, 237,
        ];
        let system_id_playready: Vec<u8> = vec![
            154, 4, 240, 121, 152, 64, 66, 134, 171, 146, 230, 91, 224, 136, 95, 149,
        ];
        let kid = None;
        let pssh = None;
        let scheme = None;

        MP4Parser::default()
            ._box("moov", Arc::new(mp4parser::children))
            ._box("trak", Arc::new(mp4parser::children))
            ._box("mdia", Arc::new(mp4parser::children))
            ._box("minf", Arc::new(mp4parser::children))
            ._box("stbl", Arc::new(mp4parser::children))
            .full_box("stsd", Arc::new(mp4parser::sample_description))
            .full_box(
                "pssh",
                Arc::new(|mut _box| {
                    if !(_box.version == 0 || _box.version == 1) {
                        return Err("PSSH version can only be 0 or 1".to_owned());
                    }

                    let system_id = _box.reader.read_bytes(16);

                    if system_id_widevine.eq(&system_id) {
                        let data_size = _box.reader.read_u32();
                        pssh = Some(openssl::base64::encode_block(
                            &_box.reader.read_bytes(data_size as usize),
                        ));
                    }

                    Ok(())
                }),
            )
            .full_box(
                "encv",
                mp4parser::alldata(Arc::new(|data| read_box(data, &mut kid, &mut scheme))),
            )
            .full_box(
                "enca",
                mp4parser::alldata(Arc::new(|data| read_box(data, &mut kid, &mut scheme))),
            )
            .full_box(
                "enct",
                mp4parser::alldata(Arc::new(|data| read_box(data, &mut kid, &mut scheme))),
            )
            .full_box(
                "encs",
                mp4parser::alldata(Arc::new(|data| read_box(data, &mut kid, &mut scheme))),
            )
            .parse(data, None, None)?;

        Ok(Self { kid, pssh, scheme })
    }
}

fn read_box(
    data: Vec<u8>,
    kid: &mut Option<String>,
    scheme: &mut Option<String>,
) -> Result<(), String> {
    let schm_bytes: Vec<u8> = vec![115, 99, 104, 109];
    let mut schm_index = 0;

    for i in 0..(data.len() - 4) {
        if data.get(i..(i + 3)) == Some(&schm_bytes) {
            schm_index = i;
        }
    }

    if (schm_index + 8) < data.len() {
        if let Some(Some(scheme_data)) = data
            .get(schm_index..)
            .map(|x| x.get(8..12).map(|x| x.to_vec()))
        {
            if let Some(scheme) = scheme {
                *scheme = String::from_utf8(scheme_data).unwrap();
            }
        }
    }

    let tenc_bytes: Vec<u8> = vec![116, 101, 110, 99];
    let mut tenc_index = None;

    for i in 0..(data.len() - 4) {
        if data.get(i..(i + 3)) == Some(&tenc_bytes) {
            tenc_index = Some(i);
        }
    }

    if let Some(tenc_index) = tenc_index {
        if (tenc_index + 12) < data.len() {
            if let Some(Some(kid_data)) = data.get(tenc_index..).map(|x| x.get(12..28)) {
                if let Some(kid) = kid {
                    *kid = openssl::bn::BigNum::from_slice(kid_data)
                        .unwrap()
                        .to_hex_str()
                        .unwrap()
                        .to_string();
                }
            }
        }
    }

    Ok(())
}
