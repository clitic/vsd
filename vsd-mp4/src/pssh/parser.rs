/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/4e933116984beb630d31ce7a0b8c9bc6f8b48c06/lib/util/pssh.js
    2. https://github.com/shaka-project/shaka-packager/blob/56e227267c9091a0f65b4d92d9064dda4557f3a7/packager/tools/pssh/pssh-box.py
    3. https://github.com/shaka-project/shaka-player/blob/b441518943241693fa2df03196be6ee707c8511e/lib/dash/content_protection.js
    4. https://github.com/rlaphoenix/pywidevine/blob/master/pywidevine/pssh.py

*/

use base64::Engine;

use crate::{
    Result, bail, data, parser,
    parser::{Mp4Parser, ParsedBox},
    pssh::{playready, widevine},
};
use std::collections::HashSet;

const COMMON_SYSTEM_ID: &str = "1077efecc0b24d02ace33c1e52e2fb4b";
const PLAYREADY_SYSTEM_ID: &str = "9a04f07998404286ab92e65be0885f95";
const WIDEVINE_SYSTEM_ID: &str = "edef8ba979d64acea3c827dcd51d21ed";

/// Parse `PSSH` box from mp4 files.
#[derive(Default)]
pub struct PsshBox {
    pub data: HashSet<PsshData>,
}

impl PsshBox {
    pub fn from_init(data: &[u8]) -> Result<Self> {
        let pssh = data!(Self::default());

        Mp4Parser::new()
            .base_box("moov", parser::children)
            .base_box("moof", parser::children)
            .full_box("pssh", {
                let pssh = pssh.clone();
                move |mut _box| {
                    pssh.borrow_mut().parse(&mut _box)?;
                    Ok(())
                }
            })
            .parse(data, false, false)?;

        Ok(pssh.take())
    }

    fn parse(&mut self, box_: &mut ParsedBox) -> Result<()> {
        if box_.version.is_none() {
            bail!("PSSH boxes are full boxes and must have a valid version.");
        }

        if box_.flags.is_none() {
            bail!("PSSH boxes are full boxes and must have a valid flag.");
        }

        let box_version = box_.version.unwrap();

        if box_version > 1 {
            bail!("Unrecognized PSSH version found!");
        }

        let system_id = hex::encode(box_.reader.read_bytes_u8(16)?);

        if box_version > 0 {
            let mut data = PsshData {
                data: box_.full_data(),
                key_ids: HashSet::new(),
                system_id: if system_id == COMMON_SYSTEM_ID {
                    SystemId::Common
                } else {
                    SystemId::Other(system_id.to_owned())
                },
            };
            let num_key_ids = box_.reader.read_u32()?;

            for _ in 0..num_key_ids {
                let key_id = hex::encode(box_.reader.read_bytes_u8(16)?);
                data.key_ids.insert(KeyId(key_id));
            }

            self.data.insert(data);
        }

        let pssh_data_size = box_.reader.read_u32()?;
        let pssh_data = box_.reader.read_bytes_u8(pssh_data_size as usize)?;

        let mut data = PsshData {
            data: box_.full_data(),
            key_ids: HashSet::new(),
            system_id: match system_id.as_str() {
                PLAYREADY_SYSTEM_ID => SystemId::PlayReady,
                WIDEVINE_SYSTEM_ID => SystemId::WideVine,
                _ => SystemId::Other(system_id.to_owned()),
            },
        };

        match system_id.as_str() {
            PLAYREADY_SYSTEM_ID => data.key_ids.extend(playready::parse(&pssh_data)?),
            WIDEVINE_SYSTEM_ID => data.key_ids.extend(widevine::parse(&pssh_data)?),
            _ => (),
        }

        self.data.insert(data);
        Ok(())
    }
}

#[derive(Eq)]
pub struct PsshData {
    pub data: Vec<u8>,
    pub key_ids: HashSet<KeyId>,
    pub system_id: SystemId,
}

/// Key id parsed from `pssh` box.
#[derive(Eq, PartialEq, Hash)]
pub struct KeyId(pub String);

/// System id type parsed from `pssh` box.
#[derive(Eq, PartialEq)]
pub enum SystemId {
    Common,
    Other(String),
    PlayReady,
    WideVine,
}

impl PartialEq for PsshData {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl std::hash::Hash for PsshData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl PsshData {
    pub fn as_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.data)
    }
}

impl KeyId {
    pub fn uuid(&self) -> String {
        format!(
            "{}-{}-{}-{}-{}",
            &self.0[..8],
            &self.0[8..12],
            &self.0[12..16],
            &self.0[16..20],
            &self.0[20..]
        )
    }
}

impl std::fmt::Display for SystemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SystemId::Common => "cen",
                SystemId::Other(x) => x,
                SystemId::PlayReady => "prd",
                SystemId::WideVine => "wvd",
            }
        )
    }
}
