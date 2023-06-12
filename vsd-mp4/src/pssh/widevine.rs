use super::{KeyId, KeyIdSystemType};
use prost::Message;

include!(concat!(env!("OUT_DIR"), "/widevine.rs"));

pub(super) fn parse(data: &[u8]) -> Result<impl IntoIterator<Item = KeyId>, String> {
    let wv = WidevinePsshData::decode(data)
        .map_err(|_| "mp4parser.pssh: pssh data is not a valid widevine data.")?;

    // let protection_scheme = String::from_utf8(wv.protection_scheme().to_be_bytes().to_vec())
    //     .map_err(|_| {
    //         "mp4parser.pssh: cannot decode widevine protection_scheme as utf8 (big endian)."
    //     })?;

    Ok(wv.key_ids.into_iter().map(|x| KeyId {
        system_type: KeyIdSystemType::WideVine,
        value: hex::encode(x),
    }))
}
