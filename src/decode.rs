use thiserror::Error;
use crate::*;

/// Error type for decoding data. 
#[derive(Error, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum InvalidData {
    /// A syllable not present in the lookup table was found. 
    #[error("Unrecognized syllable")]
    Syllable, 

    /// The number of syllables was not enough to contain the checksum. Returned only when a checksum is
    /// used; empty strings are otherwise allowed. 
    #[error("Encoded data was too short")]
    TooShort, 

    /// The checksum didn't match that of the decoded data. Returned only when a checksum is used. 
    #[error("Data integrity check failed")]
    Checksum, 
}

/// Result of decoding data. 
pub type Result<T> = std::result::Result<T, InvalidData>;

/// Decodes a string using the default [settings](Checksum). 
/// 
/// Use this only if the default checksum setting was used when encoding the string. All other
/// [encoding settings](crate::Settings) are ignored when decoding. 
/// 
/// # Examples
/// 
/// ```
/// let encoded = bunk::encode(b"aftersun");
/// let decoded = bunk::decode(encoded)?;
/// 
/// assert_eq!(decoded, b"aftersun");
/// # Ok::<(), bunk::InvalidData>(())
/// ```
pub fn decode(string: impl AsRef<str>) -> Result<Vec<u8>> {
    decode_with_settings(string, Checksum::default())
}

/// Decodes a string using given checksum settings. 
/// 
/// The checksum setting must match the one used when the string was encoded. All other
/// [encoding settings](crate::Settings) are ignored when decoding. 
/// 
/// # Examples
/// 
/// ```
/// use bunk::{Checksum, Settings};
/// 
/// let settings = Settings {
///     checksum: Checksum::Disabled, 
///     ..Default::default()
/// };
/// let encoded = bunk::encode_with_settings(b"aftersun", settings);
/// let decoded = bunk::decode_with_settings(encoded, settings.checksum)?;
/// 
/// assert_eq!(decoded, b"aftersun");
/// # Ok::<(), bunk::InvalidData>(())
/// ```
pub fn decode_with_settings(string: impl AsRef<str>, checksum: Checksum) -> Result<Vec<u8>> {
    // factored out non-generic code to reduce code size
    decode_mono(string.as_ref(), checksum)
}

/// Monomorphised decode implementation. 
#[inline(never)]
fn decode_mono(mut string: &str, checksum: Checksum) -> Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(string.len() / 2);

    // decode raw bytes from string. the bytes are still run-encoded and may have a checksum at the end
    while !string.is_empty() {
        // find the longest valid syllable at the beginning of the string
        let (index, length) = syllables::longest_prefix_of(string)
            .ok_or(InvalidData::Syllable)?;

        // the index of the syllable is its payload
        buffer.push(index);

        // gobble until start of next syllable
        string = &string[length..];
        string = string
            .find(char::is_alphabetic)
            .map(|index| string.split_at(index))
            .map(|(_, next)| next)
            .unwrap_or("");
    }

    // compute the number of bytes constituting the payload vs checksum
    let payload_len = buffer
        .len()
        .checked_sub(checksum.len())
        .ok_or(InvalidData::TooShort)?;

    // decode the payload bytes and compute their hash
    let mut hash = Fnv1a::new();

    for (i, byte) in buffer.iter_mut().enumerate().take(payload_len) {
        *byte = running_code(*byte, i);
        hash.update(*byte);
    }

    // remove checksum from the end and check whether it matches hash
    let checksum_match = buffer
        .drain(payload_len..)
        .zip(hash.bytes())
        .all(|(a, b)| a == b);

    // if so, return the fully decoded payload bytes
    checksum_match
        .then_some(buffer)
        .ok_or(InvalidData::Checksum)
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn outliers() {
        let test = |input| {
            decode_with_settings(input, Checksum::Disabled).unwrap();
        };
        test("uuuuuuuuuuu");
        test("u  u  u  u  u  u  u  u  u  u  u  ");
        test("sive123sive@tive  😀😀😀😀 son👀");
    }

    #[test]
    fn syllable_err() {
        let test = |input| {
            let result = decode_with_settings(input, Checksum::Disabled);
            assert_eq!(result, Err(InvalidData::Syllable));
        };
        test("😀");
        test("b");
        test("siv");
        test("faevlesa");
    }

    #[test]
    fn too_short_err() {
        let test = |input, checksum| {
            let result = decode_with_settings(input, checksum);
            assert_eq!(result, Err(InvalidData::TooShort));
        };
        test("",     Checksum::Length1);
        test("sive", Checksum::Length2);
        test("uu",   Checksum::Length3);
    }
}
