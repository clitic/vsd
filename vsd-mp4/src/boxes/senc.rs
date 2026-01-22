use crate::{ParsedBox, Result};

/// A subsample entry from the senc box.
#[derive(Debug, Clone)]
pub struct SencSubsample {
    /// Number of clear (unencrypted) bytes.
    pub bytes_of_clear_data: u16,
    /// Number of encrypted bytes.
    pub bytes_of_encrypted_data: u32,
}

/// Sample encryption information for a single sample.
#[derive(Debug, Clone)]
pub struct SencSample {
    /// The initialization vector for this sample.
    pub iv: Vec<u8>,
    /// Subsample encryption entries (if present).
    pub subsamples: Vec<SencSubsample>,
}

/// Sample Encryption Box (senc) - contains per-sample encryption info.
///
/// This box provides the IV (initialization vector) and optional subsample
/// encryption mapping for each sample in the fragment.
#[derive(Debug, Clone)]
pub struct SencBox {
    /// Flags from the full box header.
    pub flags: u32,
    /// Per-sample encryption information.
    pub samples: Vec<SencSample>,
}

impl SencBox {
    /// Parse a senc box from a ParsedBox.
    ///
    /// # Arguments
    /// * `box_` - The parsed box to read from
    /// * `iv_size` - The IV size (from tenc per_sample_iv_size or default)
    /// * `constant_iv` - Optional constant IV (for CBCS when per_sample_iv_size is 0)
    pub fn new(box_: &mut ParsedBox, iv_size: u8, constant_iv: Option<&[u8]>) -> Result<Self> {
        let reader = &mut box_.reader;
        let flags = box_.flags.unwrap_or(0);

        let sample_count = reader.read_u32()?;
        let has_subsamples = flags & 0x02 != 0;

        let mut samples = Vec::with_capacity(sample_count as usize);

        for _ in 0..sample_count {
            // Read per-sample IV or use constant IV
            let iv = if iv_size > 0 {
                reader.read_bytes_u8(iv_size as usize)?
            } else if let Some(civ) = constant_iv {
                civ.to_vec()
            } else {
                vec![0u8; 8] // Default 8-byte zero IV
            };

            // Read subsamples if present
            let subsamples = if has_subsamples {
                let subsample_count = reader.read_u16()?;
                let mut subs = Vec::with_capacity(subsample_count as usize);
                for _ in 0..subsample_count {
                    let bytes_of_clear_data = reader.read_u16()?;
                    let bytes_of_encrypted_data = reader.read_u32()?;
                    subs.push(SencSubsample {
                        bytes_of_clear_data,
                        bytes_of_encrypted_data,
                    });
                }
                subs
            } else {
                Vec::new()
            };

            samples.push(SencSample { iv, subsamples });
        }

        Ok(Self { flags, samples })
    }

    /// Check if this senc box has subsample encryption info.
    pub fn has_subsamples(&self) -> bool {
        self.flags & 0x02 != 0
    }

    /// Get the number of samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}
