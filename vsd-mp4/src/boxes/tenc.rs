/*
    Track Encryption Box (tenc) - contains default encryption parameters.

    REFERENCES
    ----------
    1. ISO/IEC 23001-7 (Common Encryption)
    2. https://github.com/shaka-project/shaka-player/blob/main/lib/util/mp4_box_parsers.js
*/

use crate::{Mp4Parser, ParsedBox, Result, data, parser};

/// Track Encryption Box (tenc) - default encryption parameters for a track.
///
/// This box is found in the protection scheme info (sinf/schi) and provides
/// the default KID, IV size, and pattern encryption parameters.
#[derive(Debug, Clone)]
pub struct TencBox {
    /// The default Key ID for this track.
    pub default_kid: [u8; 16],
    /// Whether the track is protected (encrypted).
    pub is_protected: bool,
    /// Per-sample IV size (0 means constant IV is used, typically for CBCS).
    pub per_sample_iv_size: u8,
    /// Number of 16-byte blocks to encrypt in pattern mode (CENS/CBCS).
    /// Only present in tenc version 1.
    pub crypt_byte_block: u8,
    /// Number of 16-byte blocks to skip in pattern mode (CENS/CBCS).
    /// Only present in tenc version 1.
    pub skip_byte_block: u8,
    /// Constant IV for CBCS mode (when per_sample_iv_size is 0).
    pub constant_iv: Option<Vec<u8>>,
}

impl TencBox {
    /// Parse a tenc box from init segment data.
    ///
    /// This method navigates through the MP4 box hierarchy to find and parse
    /// the tenc box.
    pub fn from_init(data: &[u8]) -> Result<Option<Self>> {
        let tenc_box = data!();
        let tenc_box_c = tenc_box.clone();

        Mp4Parser::new()
            .base_box("moov", parser::children)
            .base_box("trak", parser::children)
            .base_box("mdia", parser::children)
            .base_box("minf", parser::children)
            .base_box("stbl", parser::children)
            .full_box("stsd", parser::sample_description)
            .base_box("encv", parser::visual_sample_entry)
            .base_box("enca", parser::audio_sample_entry)
            .base_box("sinf", parser::children)
            .base_box("schi", parser::children)
            .full_box("tenc", move |mut box_| {
                *tenc_box_c.borrow_mut() = Some(Self::new(&mut box_)?);
                Ok(())
            })
            .parse(data, true, false)?;

        Ok(tenc_box.take())
    }

    /// Parse a tenc box from a ParsedBox.
    pub fn new(box_: &mut ParsedBox) -> Result<Self> {
        let reader = &mut box_.reader;
        let version = box_.version.unwrap_or(0);

        // Skip first reserved byte
        reader.skip(1)?;

        // In version 0, skip another reserved byte
        // In version 1, read crypt/skip pattern blocks
        let (crypt_byte_block, skip_byte_block) = if version == 0 {
            reader.skip(1)?;
            (0, 0)
        } else {
            let pattern = reader.read_u8()?;
            ((pattern >> 4) & 0x0F, pattern & 0x0F)
        };

        let is_protected = reader.read_u8()? != 0;
        let per_sample_iv_size = reader.read_u8()?;

        let kid_bytes = reader.read_bytes_u8(16)?;
        let mut default_kid = [0u8; 16];
        default_kid.copy_from_slice(&kid_bytes);

        // Read constant IV if per_sample_iv_size is 0 (CBCS mode)
        let constant_iv = if per_sample_iv_size == 0 {
            if let Ok(constant_iv_size) = reader.read_u8() {
                if constant_iv_size > 0 && constant_iv_size <= 16 {
                    reader.read_bytes_u8(constant_iv_size as usize).ok()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            default_kid,
            is_protected,
            per_sample_iv_size,
            crypt_byte_block,
            skip_byte_block,
            constant_iv,
        })
    }

    /// Get the default KID as a hex string.
    pub fn default_kid_hex(&self) -> String {
        hex::encode(self.default_kid)
    }

    /// Check if pattern encryption is used (CENS or CBCS).
    pub fn has_pattern(&self) -> bool {
        self.crypt_byte_block > 0 || self.skip_byte_block > 0
    }

    /// Get the effective IV size for decryption.
    ///
    /// Returns the per_sample_iv_size if non-zero, otherwise the constant IV size.
    pub fn effective_iv_size(&self) -> u8 {
        if self.per_sample_iv_size > 0 {
            self.per_sample_iv_size
        } else {
            self.constant_iv
                .as_ref()
                .map(|v| v.len() as u8)
                .unwrap_or(16)
        }
    }
}
