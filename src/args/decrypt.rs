use anyhow::{anyhow, Result};
use clap::{ArgEnum, Args};
use openssl::symm::{decrypt, Cipher};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, ArgEnum)]
pub enum EncryptionMethod {
    AES_128,
    CENC,
}

// opensll mp4decrypt

/// Decrypt encrypted streams using keys.
#[derive(Debug, Clone, Args)]
pub struct Decrypt {
    /// Files path to decrypt.
    #[clap(required = true)]
    pub file: String,

    /// Decrypted output file path.
    #[clap(short, long, required = true)]
    pub output: String,

    /// Encrpytion method to use.
    #[clap(short, long, arg_enum, required = true)]
    pub encryption: EncryptionMethod,

    /// Decryption KEY.
    /// KID is track ID in decimal or a 128-bit KID in hex.
    /// KEY is 128-bit key in hex.
    /// For dcf files, use 1 as the track index.
    /// For Marlin IPMP/ACGK, use 0 as the track ID.
    /// KIDs are only applicable to some encryption methods like MPEG-CENC.
    /// Example `--key edef8ba9-79d6-4ace-a3c8-27dcd51d21ed:100b6c20940f779a4589152b57d2dacb"
    /// This option can be used multiple times.
    #[clap(
        long,
        required = true,
        multiple_occurrences = true,
        value_name = "<KID:KEY>|KEY"
    )]
    pub key: Vec<String>,

    /// Initialization vector used while encrypting file.
    #[clap(long)]
    pub iv: Option<String>,

    /// Decrypt the fragments read from file, with track info read from this file.
    #[clap(long)]
    pub fragments_info: Option<String>,
}

impl Decrypt {
    pub fn perform(&self) -> Result<()> {
        let data = std::fs::read(&self.file)?;
        let mut output = File::create(&self.output)?;
        let fragments_info = if let Some(fragments_info) = &self.fragments_info {
            Some(std::fs::read(fragments_info)?)
        } else {
            None
        };
        let fragments_info = if let Some(fragments_info) = &fragments_info {
            Some(fragments_info.as_slice())
        } else {
            None
        };

        match &self.encryption {
            EncryptionMethod::AES_128 => {
                let key = self.key[0].clone();

                output.write_all(&decrypt(
                    Cipher::aes_128_cbc(),
                    key.as_bytes(),
                    self.iv.as_ref().map(|x| x.as_bytes()),
                    &data,
                )?)?;
            }
            EncryptionMethod::CENC => {
                let mut keys = HashMap::new();

                for key in &self.key {
                    keys.insert(
                        key.split(':').next().unwrap().replace("-", ""),
                        key.split(':').nth(1).unwrap_or("").to_owned(),
                    );
                }

                output.write_all(
                    &mp4decrypt::mp4decrypt(&data, keys, fragments_info).map_err(|x| anyhow!(x))?,
                )?;
            }
        }

        Ok(())
    }
}
