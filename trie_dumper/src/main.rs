use std::{fs::File, io::Write, iter};

struct CodeMapper {
    table: Vec<u32>, 
    alphabet_size: u32, 
}

struct Node {
    base: u32, 
    check: u32, 
}

struct Trie {
    mapper: CodeMapper, 
    nodes: Vec<Node>, 
}

fn main() {
    const SYLLABLES: [&[u8]; 256] = include!("../../static/syllables.txt");

    let mut keys: Vec<_> = SYLLABLES
        .iter()
        .map(|ascii| std::str::from_utf8(ascii).unwrap())
        .enumerate()
        .map(|(i, s)| (s, i as u32))
        .collect();
    keys.sort_by_key(|(key, _)| *key);

    let trie = crawdad::Trie::from_records(keys).unwrap();
    let trie: Trie = unsafe {
        std::mem::transmute(trie)
    };

    // since there is no guarantee that the memory layouts of our trie and crawdad's trie are identical, we
    // use some known values as canaries. if these fail, you're going to have to modify crawdad's source to
    // dump the data
    assert_eq!(trie.mapper.table['a' as usize], 2);
    assert_eq!(trie.mapper.table['z' as usize], 21);
    assert_eq!(trie.mapper.alphabet_size, 27);
    assert_eq!(trie.nodes[0].base, 0);
    assert_eq!(trie.nodes[0].check, u32::MAX >> 1);

    // dump code mapper table
    {
        let table: Vec<_> = trie.mapper.table[b'a' as usize ..= b'z' as usize]
            .iter()
            .copied()
            .map(|x| x as u8)
            .collect();
        File::create("../static/translation.bin")
            .and_then(|mut file| file.write_all(&table))
            .unwrap();
    }

    // dump base and check arrays
    {
        let (base, check): (Vec<_>, Vec<_>) = trie.nodes
            .iter()
            .map(|node| (node.base, node.check))
            .flat_map(|(base, check)| iter::zip(base.to_le_bytes(), check.to_le_bytes()))
            .unzip();
        File::create("../static/dart_base.bin")
            .and_then(|mut file| file.write_all(&base))
            .unwrap();
        File::create("../static/dart_check.bin")
            .and_then(|mut file| file.write_all(&check))
            .unwrap();
    }
}
