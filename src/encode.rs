use crate::*;

/// Settings used when encoding. 
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Settings {
    /// Maximum number of syllables allowed in a word. Note that the actual number of syllables in a word may
    /// be smaller. Default: `3`. 
    pub word_len: Option<u8>, 
    /// The checksum settings used. Default: [`Checksum::Length1`]. 
    pub checksum: Checksum, 
    /// If enabled, encoded strings are decorated with commas, periods, and sentence casing. This can make 
    /// the encoded string more readable, but also longer. All decorations are ignored when decoding.
    /// Default: `false`. 
    pub decorate: bool, 
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            word_len: Some(3), 
            checksum: Checksum::default(), 
            decorate: false, 
        }
    }
}

/// Encodes data using the default [settings](Settings). 
pub fn encode(data: impl AsRef<[u8]>) -> String {
    encode_with_settings(data, Settings::default())
}

/// Encodes data using given [settings](Settings). 
/// 
/// Note that the checksum settings used when decoding must match the ones used here. 
pub fn encode_with_settings(data: impl AsRef<[u8]>, settings: Settings) -> String {
    // factored out non-generic code to reduce code size
    encode_mono(data.as_ref(), settings)
}

/// Monomorphised encode implementation.  
#[inline(never)]
fn encode_mono(data: &[u8], settings: Settings) -> String {
    let Settings{ word_len: max_word, checksum, decorate } = settings;
    
    let mut sentence = Sentence {
        buffer: Vec::with_capacity(3 * (data.len() + checksum.len())), 
        previous: None, 
        word_len: 0, 
        max_word: max_word.unwrap_or(u8::MAX), 
        decorate, 
    };
    let mut hash = Fnv1a::new();

    // encode the payload
    for (i, &byte) in data.iter().enumerate() {
        hash.update(byte);
        let encoded = running_code(byte, i);
        sentence.push(encoded, hash);
    }

    let checksum_len = checksum.len();
    let checksum_bytes = hash.bytes();
    
    // encode the checksum
    for &byte in &checksum_bytes[..checksum_len] {
        // the hash is updated here only to be used as seed for the sentence encoder
        hash.update(byte);
        sentence.push(byte, hash);
    }

    let buffer = sentence.finalise();
    String::from_utf8(buffer).expect("All syllables are valid UTF-8")
}

/// Encodes bytes as a string of syllables one-by-one. 
/// 
/// Does not perform the [`running_code`] or compute a checksum; that is handled in [`encode_mono`]. 
struct Sentence {
    /// Encode ascii-string so far. 
    buffer: Vec<u8>, 
    /// Previous syllable. Used for detecting ambiguity. 
    previous: Option<&'static [u8]>, 
    /// Current word length in syllables. 
    word_len: u8, 
    /// Maximum allowed word length in syllables. 
    max_word: u8, 
    /// Whether the sentence should be decorated with periods, commas, and sentence casing. 
    decorate: bool, 
}

impl Sentence {
    /// Encodes a single byte. The seed is used to inform whether a word-break space should be replaced with
    /// a comma or period. 
    fn push(&mut self, byte: u8, seed: Fnv1a) {
        // get syllable corresponding to byte and determine whether we need a word-break before we add it
        let syllable = syllables::get(byte);
        let ambiguous = |preceding| {
            // there is a parsing ambiguity if the first char of the next syllable is a valid continuation of
            // the previous syllable
            let next = syllable[0];
            syllables::char_follows(next, preceding)
        }; 
        let word_break = self.word_len >= self.max_word || self.previous.is_some_and(ambiguous);
        
        let seed = seed.0.count_ones();
        let (capitalise, delim): (bool, Option<&[u8]>) = match (word_break, self.decorate) {
            // if we're decorating, replace a word-break space with a period or comma with some probability
            (true, true) if seed > 19 => (true,  Some(b". ")), 
            (true, true) if seed < 14 => (false, Some(b", ")), 
            // else, just use a space if we need a word-break
            (true, _)      => (false, Some(b" ")), 
            (false, true)  => (self.buffer.is_empty(), None), 
            (false, false) => (false, None), 
        };

        // if there's a delimiter (e.g. space or comma), add it before the syllable and reset ambiguity
        // control vars
        if let Some(delim) = delim {
            self.word_len = 0;
            self.previous = None;
            self.buffer.reserve(delim.len() + syllable.len());
            self.buffer.extend_from_slice(delim);
        }
        self.buffer.extend_from_slice(syllable);
        self.previous = Some(syllable);
        self.word_len += 1;
        
        if capitalise {
            let first = self.buffer.len() - syllable.len();
            self.buffer[first] = self.buffer[first].to_ascii_uppercase();
        }
    }

    /// Performs final decorations, should there be any, and returns the encoded ascii string. 
    fn finalise(mut self) -> Vec<u8> {
        if self.decorate && !self.buffer.is_empty() {
            self.buffer.push(b'.');
        }
        self.buffer
    }
}
