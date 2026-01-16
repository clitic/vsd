/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/4e933116984beb630d31ce7a0b8c9bc6f8b48c06/lib/util/pssh.js
    2. https://github.com/shaka-project/shaka-packager/blob/56e227267c9091a0f65b4d92d9064dda4557f3a7/packager/tools/pssh/pssh-box.py
    3. https://github.com/shaka-project/shaka-player/blob/b441518943241693fa2df03196be6ee707c8511e/lib/dash/content_protection.js
    4. https://github.com/rlaphoenix/pywidevine/blob/master/pywidevine/pssh.py

*/

use super::{playready, widevine};
use crate::{
    Result, bail, parser,
    parser::{Mp4Parser, ParsedBox},
};
use std::sync::{Arc, Mutex};

const COMMAN_SYSTEM_ID: &str = "1077efecc0b24d02ace33c1e52e2fb4b";
const PLAYREADY_SYSTEM_ID: &str = "9a04f07998404286ab92e65be0885f95";
const WIDEVINE_SYSTEM_ID: &str = "edef8ba979d64acea3c827dcd51d21ed";

/// Key id parsed from `pssh` box.
#[derive(Clone)]
pub struct KeyId {
    pub system_type: KeyIdSystemType,
    pub value: String,
}

impl KeyId {
    pub fn uuid(&self) -> String {
        format!(
            "{}-{}-{}-{}-{}",
            self.value.get(..8).unwrap(),
            self.value.get(8..12).unwrap(),
            self.value.get(12..16).unwrap(),
            self.value.get(16..20).unwrap(),
            self.value.get(20..).unwrap()
        )
    }
}

/// System id type parsed from `pssh` box.
#[derive(Clone)]
pub enum KeyIdSystemType {
    Comman,
    Other(String),
    PlayReady,
    WideVine,
}

impl std::fmt::Display for KeyIdSystemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                KeyIdSystemType::Comman => "comman",
                KeyIdSystemType::Other(x) => x,
                KeyIdSystemType::PlayReady => "playready",
                KeyIdSystemType::WideVine => "widevine",
            }
        )
    }
}

/// Parse `PSSH` box from mp4 files.
pub struct Pssh {
    pub key_ids: Vec<KeyId>,
    /// In hex.
    pub system_ids: Vec<String>,
}

impl Pssh {
    pub fn new(data: &[u8]) -> Result<Self> {
        let pssh = Arc::new(Mutex::new(Self {
            system_ids: vec![],
            key_ids: vec![],
        }));
        let pssh_c = pssh.clone();

        Mp4Parser::default()
            .base_box("moov", Arc::new(parser::children))
            .base_box("moof", Arc::new(parser::children))
            .full_box(
                "pssh",
                Arc::new(move |mut _box| pssh_c.lock().unwrap().parse_pssh_box(&mut _box)),
            )
            .parse(data, false, false)?;

        let pssh = pssh.lock().unwrap();
        let mut key_ids: Vec<KeyId> = vec![];

        for key_id in &pssh.key_ids {
            if !key_ids.iter().any(|x| x.value == key_id.value) {
                key_ids.push(key_id.clone())
            }
        }

        Ok(Self {
            key_ids,
            system_ids: pssh.system_ids.clone(),
        })
    }

    fn parse_pssh_box(&mut self, _box: &mut ParsedBox) -> Result<()> {
        if _box.version.is_none() {
            bail!("PSSH boxes are full boxes and must have a valid version.");
        }

        if _box.flags.is_none() {
            bail!("PSSH boxes are full boxes and must have a valid flag.");
        }

        let _box_version = _box.version.unwrap();

        if _box_version > 1 {
            // println!("Unrecognized PSSH version found!");
            return Ok(());
        }

        // The "reader" gives us a view on the payload of the box.  Create a new
        // view that contains the whole box.
        // let mut data_view = _box.reader.clone();
        // assert!(
        //     data_view.get_position() >= 12,
        //     "DataView at incorrect position"
        // );
        // self.data = view(_box.reader.clone(), - 12, _box.size as i64);

        let system_id = hex::encode(_box.reader.read_bytes_u8(16)?);

        if _box_version > 0 {
            let num_key_ids = _box.reader.read_u32()?;

            for _ in 0..num_key_ids {
                let key_id = hex::encode(_box.reader.read_bytes_u8(16)?);
                self.key_ids.push(KeyId {
                    value: key_id,
                    system_type: if system_id == COMMAN_SYSTEM_ID {
                        KeyIdSystemType::Comman
                    } else {
                        KeyIdSystemType::Other(system_id.clone())
                    },
                });
            }
        }

        let pssh_data_size = _box.reader.read_u32()?;
        let pssh_data = _box.reader.read_bytes_u8(pssh_data_size as usize)?;

        match system_id.as_str() {
            PLAYREADY_SYSTEM_ID => self.key_ids.extend(playready::parse(&pssh_data)?),
            WIDEVINE_SYSTEM_ID => self.key_ids.extend(widevine::parse(&pssh_data)?),
            _ => (),
        }

        self.system_ids.push(system_id);

        Ok(())
    }
}
