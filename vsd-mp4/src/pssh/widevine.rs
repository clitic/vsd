use crate::{
    Result,
    pssh::{KeyId, KeyIdSystemType},
};
use prost::Message;
use std::collections::HashSet;

include!(concat!(env!("OUT_DIR"), "/widevine.rs"));

pub fn parse(data: &[u8]) -> Result<HashSet<KeyId>> {
    let wv = WidevinePsshData::decode(data)?;

    Ok(wv
        .key_ids
        .into_iter()
        .map(|x| KeyId {
            system_type: KeyIdSystemType::WideVine,
            value: hex::encode(x),
        })
        .collect())
}
