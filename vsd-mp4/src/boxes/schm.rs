use crate::{ParsedBox, Result};

/// Scheme Type Box (schm) - identifies the protection scheme.
///
/// The scheme type indicates which encryption scheme is used:
/// - `cenc` (0x63656E63) - AES-CTR full sample encryption
/// - `cens` (0x63656E73) - AES-CTR subsample encryption  
/// - `cbc1` (0x63626331) - AES-CBC full sample encryption
/// - `cbcs` (0x63626373) - AES-CBC pattern encryption
pub struct SchmBox {
    /// The scheme type as a 4-byte code (e.g., 'cenc', 'cbcs').
    pub scheme_type: u32,
    /// The version of the scheme.
    pub scheme_version: u32,
    /// Optional scheme URI (if flags & 0x000001).
    pub scheme_uri: Option<String>,
}

impl SchmBox {
    /// Parse a schm box from a ParsedBox.
    pub fn new(box_: &mut ParsedBox) -> Result<Self> {
        let reader = &mut box_.reader;
        let flags = box_.flags.unwrap_or(0);

        let scheme_type = reader.read_u32()?;
        let scheme_version = reader.read_u32()?;

        // If flags indicate scheme_uri is present
        let scheme_uri = if flags & 0x000001 != 0 {
            // Read remaining bytes as URI string
            let remaining = (reader.get_length() - reader.get_position()) as usize;
            if remaining > 0 {
                let bytes = reader.read_bytes_u8(remaining)?;
                // Remove null terminator if present
                let s = String::from_utf8_lossy(&bytes);
                Some(s.trim_end_matches('\0').to_string())
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            scheme_type,
            scheme_version,
            scheme_uri,
        })
    }

    /// Check if this is CENC (AES-CTR full sample).
    pub fn is_cenc(&self) -> bool {
        self.scheme_type == 0x63656E63
    }

    /// Check if this is CENS (AES-CTR subsample/pattern).
    pub fn is_cens(&self) -> bool {
        self.scheme_type == 0x63656E73
    }

    /// Check if this is CBC1 (AES-CBC full sample).
    pub fn is_cbc1(&self) -> bool {
        self.scheme_type == 0x63626331
    }

    /// Check if this is CBCS (AES-CBC pattern).
    pub fn is_cbcs(&self) -> bool {
        self.scheme_type == 0x63626373
    }

    /// Check if this uses CTR mode (CENC or CENS).
    pub fn is_ctr_mode(&self) -> bool {
        self.is_cenc() || self.is_cens()
    }

    /// Check if this uses CBC mode (CBC1 or CBCS).
    pub fn is_cbc_mode(&self) -> bool {
        self.is_cbc1() || self.is_cbcs()
    }
}
