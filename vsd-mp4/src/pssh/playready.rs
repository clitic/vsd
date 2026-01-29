/*
    REFERENCES
    ----------

    1. https://learn.microsoft.com/en-us/playready/specifications/playready-header-specification

*/

use crate::{
    Error, Reader, Result, bail,
    pssh::{KeyId, SystemId},
};
use base64::Engine;
use serde::Deserialize;
use std::collections::HashSet;

pub fn parse(data: &[u8]) -> Result<HashSet<KeyId>> {
    let mut reader = Reader::new_little_endian(data);
    let size = reader.read_u32()?;

    if size as usize != data.len() {
        bail!("Invalid length of PSSH box playready object.");
    }

    let count = reader.read_u16()?;
    let mut kids = HashSet::new();

    for _ in 0..count {
        let record_type = reader.read_u16()?;
        let record_len = reader.read_u16()?;
        let record_data = reader.read_bytes_u16(record_len as usize)?;

        match record_type {
            1 => {
                let xml = String::from_utf16(&record_data)?;
                let wrm_header = quick_xml::de::from_str::<WrmHeader>(&xml)
                    .map_err(|x| Error::XmlDecode { error: x, xml })?;
                kids.extend(wrm_header.kids()?);
            }
            2 | 3 => (),
            _ => {
                bail!("Invalid PSSH box playready object record type {record_type}.");
            }
        }
    }

    if reader.has_more_data() {
        bail!("PSSH box extra data after playready object records.");
    }

    Ok(kids.into_iter().map(|x| KeyId(x)).collect())
}

#[derive(Deserialize)]
#[serde(rename = "WRMHEADER")]
pub struct WrmHeader {
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "DATA")]
    data: Option<Data>,
}

#[derive(Deserialize)]
pub struct Data {
    #[serde(rename = "KID")]
    kid: Option<String>,
    #[serde(rename = "PROTECTINFO")]
    protect_info: Option<ProtectInfo>,
}

#[derive(Deserialize)]
pub struct ProtectInfo {
    #[serde(rename = "KID")]
    kid: Option<KeyID>,
    #[serde(rename = "KIDS", default)]
    kids: Option<KeyIDs>,
}

#[derive(Deserialize)]
pub struct KeyID {
    #[serde(rename = "@VALUE")]
    value: String,
}

#[derive(Deserialize)]
pub struct KeyIDs {
    #[serde(rename = "KID", default)]
    kids: Vec<KeyID>,
}

impl WrmHeader {
    pub fn kids(&self) -> Result<HashSet<String>> {
        let mut kids = HashSet::new();

        match self.version.as_str() {
            "4.0.0.0" => {
                if let Some(Data { kid: Some(x), .. }) = &self.data {
                    kids.insert(x.to_owned());
                }
            }
            "4.1.0.0" => {
                if let Some(Data {
                    protect_info: Some(ProtectInfo { kid: Some(x), .. }),
                    ..
                }) = &self.data
                {
                    kids.insert(x.value.to_owned());
                }
            }
            "4.2.0.0" | "4.3.0.0" => {
                if let Some(Data {
                    protect_info: Some(ProtectInfo { kid: Some(x), .. }),
                    ..
                }) = &self.data
                {
                    kids.insert(x.value.to_owned());
                }

                if let Some(Data {
                    protect_info: Some(ProtectInfo { kids: Some(x), .. }),
                    ..
                }) = &self.data
                {
                    for kid in &x.kids {
                        kids.insert(kid.value.to_owned());
                    }
                }
            }

            x => {
                bail!("Unsupported PSSH box playready object header version v{x}.");
            }
        }

        Ok(kids
            .iter()
            .map(|x| hex::encode(base64::engine::general_purpose::STANDARD.decode(x).unwrap()))
            .collect())
    }
}
