/*
    REFERENCES
    ----------

    1. https://learn.microsoft.com/en-us/playready/specifications/playready-header-specification

*/

use super::{KeyId, KeyIdSystemType};
use crate::{mp4parser::Reader, utils};
use serde::Deserialize;

pub(super) fn parse(data: &[u8]) -> Result<impl IntoIterator<Item = KeyId>, String> {
    let mut reader = Reader::new(data, true);
    let size = reader
        .read_u32()
        .map_err(|_| "mp4parser.pssh: cannot read playready object size (u32).".to_owned())?;

    if size as usize != data.len() {
        return Err("mp4parser.pssh: playready object with invalid length.".to_owned());
    }

    let count = reader.read_u16().map_err(|_| {
        "mp4parser.pssh: cannot read playready object record count (u16).".to_owned()
    })?;

    let mut kids = vec![];

    for _ in 0..count {
        let record_type = reader.read_u16().map_err(|_| {
            "mp4parser.pssh: cannot read playready object record type (u16).".to_owned()
        })?;
        let record_len = reader.read_u16().map_err(|_| {
            "mp4parser.pssh: cannot read playready object record size (u16).".to_owned()
        })?;
        let record_data = reader.read_bytes_u16(record_len as usize).map_err(|_| {
            format!(
                "mp4parser.pssh: cannot read playready object record data ({} bytes).",
                record_len
            )
        })?;

        match record_type {
            1 => {
                let xml = String::from_utf16(&record_data).map_err(|_| {
                    "mp4parser.pssh: cannot decode playready object record data as valid utf16 data (little endian).".to_owned()
                })?;
                let wrm_header = quick_xml::de::from_str::<WrmHeader>(&xml).unwrap();
                kids.append(&mut wrm_header.kids());
            }
            2 | 3 => (),
            _ => {
                return Err(format!(
                    "mp4parser.pssh: invalid playready object record type {}",
                    record_type
                ))
            }
        }
    }

    if reader.has_more_data() {
        return Err("mp4parser.pssh: extra data after playready object records.".to_owned());
    }

    Ok(kids.into_iter().map(|x| KeyId {
        system_type: KeyIdSystemType::PlayReady,
        value: x,
    }))
}

#[derive(Deserialize)]
#[serde(rename = "WRMHEADER")]
pub(super) struct WrmHeader {
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "DATA")]
    data: Option<Data>,
}

#[derive(Deserialize)]
pub(super) struct Data {
    #[serde(rename = "KID")]
    kid: Option<String>,
    #[serde(rename = "PROTECTINFO")]
    protect_info: Option<ProtectInfo>,
}

#[derive(Deserialize)]
pub(super) struct ProtectInfo {
    #[serde(rename = "KID")]
    kid: Option<KeyID>,
    #[serde(rename = "KIDS", default)]
    kids: Option<KeyIDs>,
}

#[derive(Deserialize)]
pub(super) struct KeyID {
    #[serde(rename = "@VALUE")]
    value: String,
}

#[derive(Deserialize)]
pub(super) struct KeyIDs {
    #[serde(rename = "KID", default)]
    kids: Vec<KeyID>,
}

impl WrmHeader {
    pub(super) fn kids(&self) -> Vec<String> {
        let mut kids = vec![];

        match self.version.as_str() {
            "4.0.0.0" => {
                if let Some(Data { kid: Some(x), .. }) = &self.data {
                    kids.push(x.clone());
                }
            }
            "4.1.0.0" => {
                if let Some(Data {
                    protect_info: Some(ProtectInfo { kid: Some(x), .. }),
                    ..
                }) = &self.data
                {
                    kids.push(x.value.clone());
                }
            }
            "4.2.0.0" | "4.3.0.0" => {
                if let Some(Data {
                    protect_info: Some(ProtectInfo { kid: Some(x), .. }),
                    ..
                }) = &self.data
                {
                    kids.push(x.value.clone());
                }

                if let Some(Data {
                    protect_info: Some(ProtectInfo { kids: Some(x), .. }),
                    ..
                }) = &self.data
                {
                    kids.extend(x.kids.iter().map(|x| x.value.clone()));
                }
            }

            x => panic!("unsupported playready header version v{}", x),
        }

        kids.iter()
            .map(|x| hex::encode(utils::decode_base64(x).unwrap()))
            .collect()
    }
}
