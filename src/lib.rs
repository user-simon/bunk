//! Fast and efficient human-readable data encoding! 
//! 
//! Bunk encodes binary data as pronounceable gibberish, somewhat resembling latin. This is useful when
//! binary data such as an encryption key is shown to an end-user who might need to manually transfer it. 
//! 
//! Using the default [settings](Settings), a string of 32 bytes gets encoded as: 
//! ```text
//! atemorni telphocom neideu gepypi forzamar oasal cevanal butthepo aujoate turviy menkais
//! ```
//! 
//! Optionally, Bunk can [decorate](Settings::decorate) the encoded string with commas, periods, and sentence
//! casing to improve readability: 
//! ```text
//! Atemorni telphocom. Neideu gepypi forzamar oasal cevanal butthepo aujoate turviy, menkais.
//! ```
//! 
//! 
//! # Overview
//! 
//! - It is fast! On my machine, encoding and then decoding a random array of 32 bytes takes an average of
//! ~0.8µs with the default settings --- allocations and all; no hidden fees. 
//! - It is small! Bunk stores a table of only 256 syllables, each between 1-4 letters (average of 2.47), and
//! some data structures needed for fast lookup. 
//! - Checksums of variable length can be added to encoded messages to verify data integrity when decoding. 
//! - The [maximum word length](Settings::word_len) (in syllables) can be customized. 
//! 
//! 
//! # How it compares to English dictionary encodings
//! 
//! A popular scheme is to encode binary data as actual English words, which yields results that are more
//! readable and easier to remember. See [bip39](https://docs.rs/tiny-bip39/) as an example of this. However,
//! to be efficient in the amount of data a string of words can encode, a _massive_ table of (sometimes
//! quite long) words must be included --- [bip39](https://docs.rs/tiny-bip39/) uses 2048 words. In addition
//! to this, some kind of data structure for lookup is also needed, and will likely have to be constructed at
//! runtime. 
//! 
//! Bunk takes a different approach, requiring a table of only 256 1-4 letter syllables, each carrying one
//! byte of data. This allows Bunk to: 
//! - Take up less memory overall. 
//! - Store data structures needed for fast lookup in static memory instead of having to construct it at
//! runtime. 
//! 
//! 
//! # Serde
//! 
//! Enable the `serde` feature and Bunk can be used to serialize/deserialize fields that implement
//! `AsRef<[u8]>` and `From<Vec<u8>>`: 
//! ```text
//! #[derive(Serialize, Deserialize)]
//! struct Vault {
//!     #[serde(with = "bunk")]
//!     key: Vec<u8>, 
//!     name: String, 
//! }
//! ```
//! 
//! Note that the [settings](Settings) used when encoding for serde are necessarily hard-coded: 
//! ```no_run
//! # use bunk::*;
//! # let _ =
//! Settings {
//!     word_len: Some(3), 
//!     checksum: Checksum::Disabled, 
//!     decorate: false, 
//! }
//! # ;
//! ```
//! 
//! 
//! # Examples
//! 
//! Basic usage with default [settings](Settings): 
//! ```
//! let encoded = bunk::encode(b"aftersun");
//! let decoded = bunk::decode(encoded)?;
//! 
//! assert_eq!(decoded, b"aftersun");
//! # Ok::<(), bunk::InvalidData>(())
//! ```
//! 
//! Disabled [checksum](Checksum): 
//! ```
//! use bunk::{Checksum, Settings};
//! 
//! let settings = Settings {
//!     checksum: Checksum::Disabled, 
//!     ..Default::default()
//! };
//! let encoded = bunk::encode_with_settings(b"it's such a beautiful day", settings);
//! let decoded = bunk::decode_with_settings(encoded, settings.checksum)?;
//! 
//! assert_eq!(decoded, b"it's such a beautiful day");
//! # Ok::<(), bunk::InvalidData>(())
//! ```
//! 
//! Custom [checksum length](Checksum): 
//! ```
//! use bunk::{Checksum, Settings};
//! 
//! let settings = Settings {
//!     checksum: Checksum::Length4, 
//!     ..Default::default()
//! };
//! let encoded = bunk::encode_with_settings([33, 14, 224, 134], settings);
//! let decoded = bunk::decode_with_settings(encoded, settings.checksum)?;
//! 
//! assert_eq!(decoded, [33, 14, 224, 134]);
//! # Ok::<(), bunk::InvalidData>(())
//! ```
//! 
//! Custom [word length limit](Settings::word_len): 
//! ```
//! use bunk::{Checksum, Settings};
//! 
//! let settings = Settings {
//!     word_len: Some(5), 
//!     ..Default::default()
//! };
//! let encoded = bunk::encode_with_settings([231, 6, 39, 34], settings);
//! let decoded = bunk::decode(encoded)?; // word_len doesn't affect the decoder
//! 
//! assert_eq!(decoded, [231, 6, 39, 34]);
//! # Ok::<(), bunk::InvalidData>(())
//! ```
//! 
//! 
//! # How it works
//! 
//! To explain the algorithm, we'll iteratively build upon it and solve issues as we go. 
//! 
//! The fundamental idea is to encode a byte as a syllable by using it to index into a table of 256 unique
//! syllables, the result of which is then appended to the encoded string --- as one would expect. The
//! decoder can then use a [trie](https://en.wikipedia.org/wiki/Trie) to find the index of the longest
//! syllable at the beginning of the string, which corresponds to the encoded byte. 
//! 
//! This by itself causes issues of parser ambiguity when one valid syllable is a prefix of another. Take as
//! a basic example the encoded string "ous". Is this the single syllable "ous", or the syllable "o" followed
//! by "us"? Barring some cumbersome machinery, there is no way for the decoder to know! The encoder
//! therefore has to detect when such an ambiguity is possible by checking if the first letter of the second
//! syllable is a valid continuation of the first syllable. If so, it inserts a word break between them.
//! (Technically, this is stricter than necessary for breaking the ambiguity but is easy to check and allows
//! the decoder to be written greedily.)
//! 
//! To support these two required operations --- finding the longest syllable prefixed to a string, and
//! checking whether a letter is a valid continuation of a syllable --- Bunk uses a trie. There are then two
//! issues presenting themselves: 
//! - Tries are _slow_ to construct. 
//! - There are (somehow) no efficient trie libraries for Rust that allows for these operations in their API. 
//! 
//! As a solution to both of these, a precomputed trie (as created by [crawdad](https://docs.rs/crawdad/)) is
//! stored in static memory, on top of which Bunk implements a basic traversal, which the only API needed for
//! the two operations. All in all, the trie API comes out to only about 60 lines of code --- much less than
//! having to add [crawdad](https://docs.rs/crawdad/) (or such) as a dependency. 
//! 
//! So far, the algorithm we've described is a perfectly functional encoder. However, to be more
//! user-friendly, we'd ideally also like _all_ inputs to yield equally pronounceable text. Without any
//! further measures, inputs such as `[0, 0, 0, 0]` yield repeated syllables, in this case "uuu u". To avoid
//! this, Bunk artificially increases the _apparent_ entropy of encoded bytes by first XORing them with a
//! value dependant on their index. Since XOR undoes itself, the decoder can then do the exact same thing and
//! retrieve the original bytes. With this in place, `[0, 0, 0, 0]` gets nicely encoded as "trirori mulry". 

mod encode;
mod decode;
mod syllables;
mod serde;

pub use encode::*;
pub use decode::*;

#[cfg(feature = "serde")]
pub use serde::*;

/// Specifies the number of checksum bytes used when encoding. 
/// 
/// Default: [`Checksum::Length1`]. 
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Checksum {
    /// No bytes used; the encoded data will not contain a checksum. 
    Disabled, 
    /// One byte used. 
    Length1, 
    /// Two bytes used. 
    Length2, 
    /// Three bytes used. 
    Length3, 
    /// Four bytes used. 
    Length4, 
}

impl Checksum {
    /// Returns the number of checksum bytes to be included in encoded data. 
    const fn len(self) -> usize {
        self as usize
    }
}

impl Default for Checksum {
    fn default() -> Self {
        Checksum::Length1
    }
}

/// The FNV-1a hashing algorithm. 
/// 
/// Implementation based on pseudo-code on
/// [Wikipedia](https://en.wikipedia.org/wiki/Fowler-Noll-Vo_hash_function). This is used for the checksum. 
#[derive(Clone, Copy)]
struct Fnv1a(u32);

impl Fnv1a {
    /// Creates a hasher initialised with the FNV offset basis. 
    const fn new() -> Fnv1a {
        Fnv1a(0x811c9dc5)
    }

    /// Digests one byte. 
    fn update(&mut self, byte: u8) {
        self.0 ^= byte as u32;
        self.0 = self.0.wrapping_mul(0x01000193);
    }

    /// Returns the bytes to be used as checksum. 
    const fn bytes(&self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

/// Increases _apparent_ entropy in input data. 
/// 
/// Before getting the syllable corresponding to a byte, it along with its index is run through this function
/// to reduce visible patterns in the input data. This ensures that e.g. `[0, 0, 0, 0]` gets encoded as
/// `trirori mul` and not `uuu u`. 
/// 
/// Some notes: 
/// - This neither increases nor decreases security; it is completely transparent, and used only to make the
/// output look nicer. 
/// - The transformation applied to bytes repeats every 256 indices. 
/// - This function undoes itself if the index is the same; i.e., it both encodes and decodes bytes. 
/// 
/// ```ignore
/// let input = 0xC5;
/// let encoded = running_code(input, 0);
/// let decoded = running_code(encoded, 0);
/// assert_eq!(input, decoded)
/// ```
fn running_code(byte: u8, index: usize) -> u8 {
    const TABLE: [u8; 256] = include!("../static/entropy.txt");
    byte ^ TABLE[index & 0xFF]
}

#[cfg(test)]
mod tests {
    use rand::{rngs::SmallRng, RngCore, SeedableRng};
    use crate::*;

    fn round_trip(data: &[u8], settings: Settings) {
        let encoded = super::encode_with_settings(data, settings);
        let decoded = super::decode_with_settings(&encoded, settings.checksum);
        assert_eq!(decoded.as_deref(), Ok(data), "{data:?}, {settings:?}");
    }

    fn stress(n: usize) {
        let checksums = [
            Checksum::Disabled, 
            Checksum::Length1, 
            Checksum::Length2, 
            Checksum::Length3, 
            Checksum::Length4, 
        ];
        let max_words = [None, Some(1), Some(2), Some(3), Some(10), Some(11)];
        let decorates = [true, false];
        let sizes = [0, 1, 2, 3, 10, 16, 30, 31, 32, 64, 100, 250, 509, 510];

        let stress_settings = |data: &[u8]| {
            for checksum in checksums {
                for max_word in max_words {
                    for decorate in decorates {
                        let settings = Settings {
                            checksum, 
                            word_len: max_word, 
                            decorate, 
                        };
                        round_trip(data, settings);
                    }
                }
            }
        };
        let mut rng = SmallRng::seed_from_u64(7502546294857623797);

        for size in sizes {
            for _ in 0..n {
                let mut data = vec![0; size];
                rng.fill_bytes(&mut data);
                
                stress_settings(&data);
            }
        }
    }

    #[test]
    fn stress_medium() {
        stress(500);
    }
}
