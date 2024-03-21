# 💬 Bunk. 

Fast and efficient human-readable data encoding! 

Bunk encodes binary data as pronounceable gibberish, somewhat resembling Latin. This is useful when binary
data such as an encryption key is shown to an end-user who might need to manually transfer it. 

Using the default settings, a string of 32 bytes gets encoded as: 
```
atemorni telphocom neideu gepypi forzamar oasal cevanal butthepo aujoate turviy menkais
```

Optionally, Bunk can decorate the encoded string with commas, periods, and sentence
casing to improve readability: 
```text
Atemorni telphocom. Neideu gepypi forzamar oasal cevanal butthepo aujoate turviy, menkais.
```

## 📚 Documentation

Read the documentation **[HERE](https://docs.rs/bunk)**. 


## 📋 Overview

- It is fast! On my machine, encoding and then decoding a random array of 32 bytes takes an average of
~0.8µs with the default settings — allocations and all; no hidden fees. 
- It is small! Bunk stores a table of only 256 syllables, each between 1-4 letters (average of 2.47), and
some data structures needed for fast lookup. 
- Checksums of variable length can be added to encoded messages to verify data integrity when decoding, which protects against typos. 
- The maximum word length (in syllables) can be customized. 


## ⚖️ How it compares to English dictionary encodings

A popular scheme is to encode binary data as actual English words, which yields results that are more
readable and easier to remember. See [bip39](https://docs.rs/tiny-bip39/) as an example of this. However, to
be efficient in the amount of data a string of words can encode, a _massive_ table of (sometimes quite long)
words must be included — [bip39](https://docs.rs/tiny-bip39/) uses 2048 words. In addition to this, some
kind of data structure for lookup is also needed, and will likely have to be constructed at runtime. If this 
is of no object to your application, use something like [bip39](https://docs.rs/tiny-bip39/) instead!

Bunk takes a different approach, requiring a table of only 256 1-4 letter syllables, each carrying one
byte of data. This allows Bunk to: 
- Take up less memory overall. 
- Store data structures needed for fast lookup in static memory instead of having to construct it at
runtime. 


## 🛠️ How it works

To explain the algorithm, we'll iteratively build upon it and solve issues as we go. 

The fundamental idea is to encode a byte as a syllable by using it to index into a table of 256 unique
syllables, the result of which is then appended to the encoded string — as one would expect. The decoder
can then use a [trie](https://en.wikipedia.org/wiki/Trie) to find the index of the longest syllable at the
beginning of the string, which corresponds to the encoded byte. 

This by itself causes issues of parser ambiguity when one valid syllable is a prefix of another. Take as a 
basic example the encoded string "ous". Is this the single syllable "ous", or the syllable "o" followed by 
"us"? Barring some cumbersome machinery, there is no way for the decoder to know! The encoder therefore has to detect when such an ambiguity is possible by checking if the first letter of the second syllable is a
valid continuation of the first syllable. If so, it inserts a word break between them. (Technically, this is
stricter than necessary for breaking the ambiguity but is easy to check and allows the decoder to be written
greedily.)

To support these two required operations — finding the longest syllable prefixed to a string, and checking
whether a letter is a valid continuation of a syllable — Bunk uses a trie. There are then two issues
presenting themselves: 
- Tries are _slow_ to construct. 
- There are (somehow) no efficient trie libraries for Rust that allows for these operations in their API. 

As a solution to both of these, a precomputed trie (as created by [crawdad](https://docs.rs/crawdad/)) is
stored in static memory, on top of which Bunk implements a basic traversal, which the only API needed for the
two operations. All in all, the trie API comes out to only about 60 lines of code — much less than having
to add [crawdad](https://docs.rs/crawdad/) (or such) as a dependency. 

So far, the algorithm we've described is a perfectly functional encoder. However, to be more user-friendly, 
we'd ideally also like _all_ inputs to yield equally pronounceable text. Without any further measures, inputs
such as `[0, 0, 0, 0]` yield repeated syllables, in this case "uuu u". To avoid this, Bunk artificially
increases the _apparent_ entropy of encoded bytes by first XORing them with a value dependent on their index. 
Since XOR undoes itself, the decoder can then do the exact same thing and retrieve the original bytes. With
this in place, `[0, 0, 0, 0]` gets nicely encoded as "trirori mulry". 
