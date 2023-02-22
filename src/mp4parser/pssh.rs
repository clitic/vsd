/*
    REFERENCES
    ----------

    1. https://github.com/shaka-project/shaka-player/blob/4e933116984beb630d31ce7a0b8c9bc6f8b48c06/lib/util/pssh.js

*/

use crate::mp4parser;
use crate::mp4parser::{Mp4Parser, ParsedBox};
use std::sync::Arc;

/// Parse a PSSH box and extract the system IDs.
pub struct Pssh {
    /// In hex.
    system_ids: Vec<String>,
    /// In hex.
    cenc_key_ids: Vec<String>,
    /// Array with the pssh boxes found.
    data: Vec<u8>,
}

impl Pssh {
    pub fn new(data: &[u8]) -> Result<Self, String> {
        let mut pssh = Self {
            system_ids: vec![],
            cenc_key_ids: vec![],
            data: vec![],
        };

        Mp4Parser::default()
            ._box("moov", Arc::new(mp4parser::children))
            ._box("moof", Arc::new(mp4parser::children))
            .full_box("pssh", Arc::new(|_box| pssh.parse_pssh_box(_box)))
            .parse(data, None, None)?;

        Ok(pssh)
    }

    fn parse_pssh_box(&mut self, _box: ParsedBox) -> Result<(), String> {
        assert!(
            _box.version.is_some(),
            "PSSH boxes are full boxes and must have a valid version"
        );
        assert!(
            _box.flags.is_some(),
            "PSSH boxes are full boxes and must have a valid flag"
        );

        let _box_version = _box.version.unwrap();

        if _box_version > 1 {
            // println!("Unrecognized PSSH version found!");
            return Ok(());
        }

        // The "reader" gives us a view on the payload of the box.  Create a new
        // view that contains the whole box.
        let data_view = _box.reader.clone();

        assert!(
            data_view.get_position() >= 12,
            "DataView at incorrect position"
        );

        data_view.set_position(data_view.get_position() - 12);
        self.data = data_view.read_bytes(_box.size).map_err(|_| {
            format!(
                "mp4parser: cannot read pssh box data ({} bytes).",
                _box.size
            )
        })?;

        self.system_ids
            .push(hex::encode(_box.reader.read_bytes(16).map_err(|_| {
                "mp4parser: cannot read pssh box system ids (16 bytes).".to_owned()
            })?));

        if _box_version > 0 {
            let num_key_ids = _box
                .reader
                .read_u32()
                .map_err(|_| "mp4parser: cannot read pssh box number of key ids (u32).".to_owned())?;

            for i in 0..num_key_ids {
                let key_id = hex::encode(_box.reader.read_bytes(16).map_err(|_| {
                    "mp4parser: cannot read pssh box key id (16 bytes).".to_owned()
                })?);
                self.cenc_key_ids.push(key_id);
            }
        }

        Ok(())
    }
}
