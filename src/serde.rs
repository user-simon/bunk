#![cfg(feature = "serde")]

use serde::{Deserialize, Deserializer, Serializer};
use crate::{Checksum, Settings};

/// Hard-coded settings used for Serde serialization. 
const SETTINGS: Settings = Settings {
    max_word: Some(3), 
    checksum: Checksum::Disabled, 
    decorate: false, 
};

/// Serialize data for Serde using Bunk. 
pub fn serialize<S>(data: impl AsRef<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer
{
    let string = crate::encode_with_settings(data, SETTINGS);
    serializer.serialize_str(&string)
}

/// Deserialize data from Serde using Bunk. 
pub fn deserialize<'a, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: From<Vec<u8>>, 
    D: Deserializer<'a>, 
{
    use serde::de::Error;

    let decode = |string| crate::decode_with_settings(string, SETTINGS.checksum)
        .map_err(D::Error::custom);
    
    String::deserialize(deserializer)
        .and_then(decode)
        .map(T::from)
}
