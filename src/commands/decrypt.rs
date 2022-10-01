use anyhow::{anyhow, Result};
use clap::{Args, ValueEnum};
use openssl::symm::{decrypt, Cipher};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, ValueEnum)]
enum EncryptionMethod {
    AES_128,
    CENC,
}

/// Decrypt encrypted streams using keys.
#[derive(Debug, Clone, Args)]
pub struct Decrypt {
    /// Path of file to decrypt.
    #[arg(required = true)]
    file: String,

    /// Path of decrypted output file.
    #[arg(short, long, required = true)]
    output: String,

    /// Encryption method to use.
    ///
    /// AES-128 decryption is based on openssl aes-128-cbc decryption.
    /// CENC decryption is based on mp4decrypt from Bento4 (https://github.com/axiomatic-systems/Bento4).
    #[arg(short, long, value_enum, required = true)]
    encryption: EncryptionMethod,

    /// Decryption KEY.
    /// This option can be used multiple times.
    ///
    /// ## AES-128 (CBC)
    ///
    /// KEY is either a base64 string or a file path.
    /// example: --key bin.key
    ///
    /// ## CENC (mp4decrypt)
    ///
    /// KID is track ID in decimal or a 128-bit KID in hex.
    /// KEY is 128-bit key in hex.
    ///
    /// For dcf files, use 1 as the track index.
    /// For Marlin IPMP/ACGK, use 0 as the track ID.
    /// KIDs are only applicable to some encryption methods like MPEG-CENC.
    ///
    /// example: --key edef8ba9-79d6-4ace-a3c8-27dcd51d21ed:100b6c20940f779a4589152b57d2dacb
    #[arg(short, long, required = true, value_name = "KEY|<KID:KEY>")]
    key: Vec<String>,

    /// Initialization vector used while encrypting file.
    #[arg(long)]
    iv: Option<String>,

    /// Decrypt the fragments read from file, with track info read from this file.
    #[arg(long)]
    fragments_info: Option<String>,
}

impl Decrypt {
    pub fn perform(&self) -> Result<()> {
        let data = match &self.encryption {
            EncryptionMethod::AES_128 => decrypt(
                Cipher::aes_128_cbc(),
                &if self.key[0].ends_with('=') {
                    base64::decode(&self.key[0])?
                } else {
                    std::fs::read(&self.key[0])?
                },
                self.iv.as_ref().map(|x| x.as_bytes()),
                &std::fs::read(&self.file)?,
            )?,
            EncryptionMethod::CENC => {
                let mut keys = HashMap::new();

                for key in &self.key {
                    keys.insert(
                        key.split(':').next().unwrap().replace('-', ""),
                        key.split(':').nth(1).unwrap_or("").to_owned(),
                    );
                }

                let fragments_info = if let Some(fragments_info) = &self.fragments_info {
                    Some(std::fs::read(fragments_info)?)
                } else {
                    None
                };

                mp4decrypt::mp4decrypt(
                    &std::fs::read(&self.file)?,
                    keys,
                    if let Some(fragments_info) = &fragments_info {
                        Some(fragments_info.as_slice())
                    } else {
                        None
                    },
                )
                .map_err(|x| anyhow!(x))?
            }
        };

        File::create(&self.output)?.write_all(&data)?;
        Ok(())
    }
}
