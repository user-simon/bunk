//! Interface for the syllables and accompanying lookups used when encoding and decoding. 

use std::iter;
use include_bytes_plus::include_bytes;

/// Gets the ascii string of a syllable identified by its index. 
pub const fn get(index: u8) -> &'static [u8] {
    const SYLLABLES: [&[u8]; 256] = include!("../static/syllables.txt");
    SYLLABLES[index as usize]
}

/// Greedily attempts to finds the longest syllable prefixed to a string. 
/// 
/// Returns `(syllable_index, syllable_length)`. 
pub fn longest_prefix_of(string: &str) -> Option<(u8, usize)> {
    let mut node = Node::root();
    let mut len = 0;

    for char in string.chars() {
        let child = char
            .try_into()
            .ok()
            .and_then(|ascii| node.child(ascii));
        let Some(child) = child else {
            break
        };
        node = child;
        len += 1;
    }
    node.syllable().map(|syllable| (syllable, len))    
}

/// Determines whether a letter is a valid continuation of a syllable, i.e., whether the letter is a valid
/// transition from the trie node of the syllable. 
pub fn char_follows(char: u8, syllable: &[u8]) -> bool {
    syllable.iter()
        .copied()
        .chain(iter::once(char))
        .try_fold(Node::root(), Node::child)
        .is_some()
}

/// Represents a node of the trie. 
/// 
/// The trie library [crawdad](https://docs.rs/crawdad) was used to generate the base and check arrays which
/// are stored in static memory, but since it doesn't allow you to step through the trie (what 
/// [`Node::child`] does), we replace it with our own basic implementation via this struct. 
/// 
/// See [double-array tries](https://www.linux.thai.net/~thep/datrie/) and the
/// [crawdad source](https://github.com/daac-tools/crawdad/blob/main/src/trie.rs), for more information
/// on how this all works. 
#[derive(Clone, Copy, Debug)]
struct Node {
    /// Index of the node. 
    id: u32, 
    /// The base of the transitions from the node. 
    base: u32, 
    /// Whether the node has any transitions. 
    is_leaf: bool, 
    /// Whether the node has a value. If [`Node::is_leaf`] is true, [`Node::base`] is the value of the node, 
    /// otherwise, [`Node::base`] is the index of the value in the base array. 
    has_value: bool, 
}

impl Node {
    /// The root node of the trie, wherefrom all lookups begin. 
    const fn root() -> Node {
        Node {
            id: 0, 
            base: base(0).1, 
            is_leaf: false, 
            has_value: false, 
        }
    }

    /// Get the index of the syllable represented by the node. 
    fn syllable(self) -> Option<u8> {
        let syllable = match (self.has_value, self.is_leaf) {
            (true, true) => Some(self.base), 
            (true, false) => Some(base(self.base).1), 
            (false, _) => None, 
        };
        syllable.map(|x| x as u8)
    }

    /// Perform a given transition to a child node. 
    fn child(self, char: u8) -> Option<Node> {
        const TRANSLATION: [u8; 26] = include_bytes!("static/translation.bin");

        // translate ascii char code to a mangled code representing the transition
        let code = char
            .to_ascii_lowercase()
            .checked_sub(b'a')
            .and_then(|code| TRANSLATION.get(code as usize))
            .map(|&code| code as u32)?;

        // compute the child node
        let id = self.base ^ code;
        let (is_leaf, base) = base(id);
        let (has_leaf, check) = check(id);
        let node = Node {
            id, 
            base, 
            is_leaf, 
            has_value: is_leaf || has_leaf, 
        };

        // verify that the transition to the child actually exists and if so, return the child
        (check == self.id).then_some(node)
    }
}

/// Splits an integer into the most significant bit and the remainder. 
/// 
/// Both [`base`] and [`check`] use the MSB as a flag so this exists as a utility to extract that. 
const fn split_msb(integer: u32) -> (bool, u32) {
    const MASK: u32 = !0 >> 1;
    (integer & !MASK != 0, integer & MASK)
}

/// Index into the base array of the [double-array trie](https://www.linux.thai.net/~thep/datrie/). 
/// 
/// Returns `(is_leaf, base)`; both are stored in the integer. 
const fn base(node_id: u32) -> (bool, u32) {
    const BASE: &[u32] = &include_bytes!("static/dart_base.bin" as u32le);
    split_msb(BASE[node_id as usize])
}

/// Index into the check array of the [double-array trie](https://www.linux.thai.net/~thep/datrie/). 
/// 
/// Returns `(has_leaf, check)`; both are stored in the integer. 
const fn check(node_id: u32) -> (bool, u32) {
    const CHECK: &[u32] = &include_bytes!("static/dart_check.bin" as u32le);
    split_msb(CHECK[node_id as usize])
}
