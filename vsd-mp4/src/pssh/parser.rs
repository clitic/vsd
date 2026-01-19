/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/4e933116984beb630d31ce7a0b8c9bc6f8b48c06/lib/util/pssh.js
    2. https://github.com/shaka-project/shaka-packager/blob/56e227267c9091a0f65b4d92d9064dda4557f3a7/packager/tools/pssh/pssh-box.py
    3. https://github.com/shaka-project/shaka-player/blob/b441518943241693fa2df03196be6ee707c8511e/lib/dash/content_protection.js
    4. https://github.com/rlaphoenix/pywidevine/blob/master/pywidevine/pssh.py

*/

use crate::{
    Result, bail, parser,
    parser::{Mp4Parser, ParsedBox},
    pssh::{playready, widevine},
};
use std::{cell::RefCell, collections::HashSet, rc::Rc};

const COMMON_SYSTEM_ID: &str = "1077efecc0b24d02ace33c1e52e2fb4b";
const PLAYREADY_SYSTEM_ID: &str = "9a04f07998404286ab92e65be0885f95";
const WIDEVINE_SYSTEM_ID: &str = "edef8ba979d64acea3c827dcd51d21ed";

/// Parse `PSSH` box from mp4 files.
#[derive(Default)]
pub struct PsshBox {
    pub key_ids: HashSet<KeyId>,
    pub system_ids: HashSet<String>,
}

impl PsshBox {
    pub fn from_init(data: &[u8]) -> Result<Self> {
        let pssh = Rc::new(RefCell::new(Self {
            key_ids: HashSet::new(),
            system_ids: HashSet::new(),
        }));
        let pssh_c = pssh.clone();

        Mp4Parser::new()
            .base_box("moov", parser::children)
            .base_box("moof", parser::children)
            .full_box("pssh", move |mut _box| {
                pssh_c.borrow_mut().parse(&mut _box)?;
                Ok(())
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

        // The "reader" gives us a view on the payload of the box.  Create a new
        // view that contains the whole box.
        // let mut data_view = _box.reader.clone();
        // assert!(
        //     data_view.get_position() >= 12,
        //     "DataView at incorrect position"
        // );
        // self.data = view(_box.reader.clone(), - 12, _box.size as i64);

        let system_id = hex::encode(box_.reader.read_bytes_u8(16)?);

        if box_version > 0 {
            let num_key_ids = box_.reader.read_u32()?;

            for _ in 0..num_key_ids {
                let key_id = hex::encode(box_.reader.read_bytes_u8(16)?);
                self.key_ids.insert(KeyId {
                    value: key_id,
                    system_type: if system_id == COMMON_SYSTEM_ID {
                        KeyIdSystemType::Common
                    } else {
                        KeyIdSystemType::Other(system_id.to_owned())
                    },
                });
            }
        }

        let pssh_data_size = box_.reader.read_u32()?;
        let pssh_data = box_.reader.read_bytes_u8(pssh_data_size as usize)?;

        match system_id.as_str() {
            PLAYREADY_SYSTEM_ID => self.key_ids.extend(playready::parse(&pssh_data)?),
            WIDEVINE_SYSTEM_ID => self.key_ids.extend(widevine::parse(&pssh_data)?),
            _ => (),
        }

        self.system_ids.insert(system_id);
        Ok(())
    }
}

/// Key id parsed from `pssh` box.
pub struct KeyId {
    pub system_type: KeyIdSystemType,
    pub value: String,
}

impl Eq for KeyId {}

impl PartialEq for KeyId {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl std::hash::Hash for KeyId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl KeyId {
    pub fn uuid(&self) -> String {
        format!(
            "{}-{}-{}-{}-{}",
            &self.value[..8],
            &self.value[8..12],
            &self.value[12..16],
            &self.value[16..20],
            &self.value[20..]
        )
    }
}

/// System id type parsed from `pssh` box.
pub enum KeyIdSystemType {
    Common,
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
                KeyIdSystemType::Common => "common",
                KeyIdSystemType::Other(x) => x,
                KeyIdSystemType::PlayReady => "playready",
                KeyIdSystemType::WideVine => "widevine",
            }
        )
    }
}
