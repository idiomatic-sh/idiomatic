//! The services of the `bytes` part: a byte term's literals numbered once a worker, and a file's bytes read for them once
//! a file.

use super::types::{ByteFile, ByteLiterals, ByteSearch};
use crate::domain::code::types::WordSlot;

/// The place of a byte term among the literals' terms, its alternatives numbered the first time it is met: `texts` are
/// the texts of which the file must hold one; none: the term asks nothing (every file may hold its texts).
pub(crate) fn term_wants(
    literals: &mut ByteLiterals,
    key: (WordSlot, WordSlot),
    texts: impl FnOnce() -> Option<Vec<Vec<u8>>>,
) -> u32 {
    if let Some(place) = literals.terms.get(&key) {
        return *place;
    }
    let numbers = texts().filter(|texts| !texts.is_empty() && texts.iter().all(|text| !text.is_empty())).map(|texts| {
        texts
            .into_iter()
            .map(|text| literal_number(literals, text))
            .collect::<Box<[u32]>>()
    });
    let place = u32::try_from(literals.wants.len()).unwrap_or(u32::MAX);
    literals.wants.push(numbers);
    let _old = literals.terms.insert(key, place);
    place
}

/// The number of a literal, given the first time it is met.
fn literal_number(literals: &mut ByteLiterals, text: Vec<u8>) -> u32 {
    if let Some(number) = literals.numbers.get(text.as_slice()) {
        return *number;
    }
    let number = u32::try_from(literals.literals.len()).unwrap_or(u32::MAX);
    let search = match text.as_slice() {
        [byte] => ByteSearch::Byte(*byte),
        longer => ByteSearch::Text(memchr::memmem::Finder::new(longer).into_owned()),
    };
    literals.literals.push(search);
    let _old = literals.numbers.insert(text.into_boxed_slice(), number);
    number
}

/// Whether the file's bytes hold one of the alternatives of the byte term at `place`; the bytes are read for the
/// literals not yet answered in this file, all of them at once. A term that asks nothing holds.
pub(crate) fn held(literals: &ByteLiterals, source: &[u8], file: &mut ByteFile, place: u32) -> bool {
    let Some(Some(wanted)) = literals.wants.get(usize::try_from(place).unwrap_or(usize::MAX)) else {
        return true;
    };
    if file.known < literals.literals.len() {
        file_read(literals, source, file);
    }
    wanted.iter().any(|number| {
        let at = usize::try_from(*number).unwrap_or(usize::MAX);
        file.held.get(at / 64).is_some_and(|word| (word >> (at % 64)) & 1 == 1)
    })
}

/// Reads the file's bytes for the literals not yet answered: the byte set once a file, then each longer literal searched
/// until its first occurrence.
pub(crate) fn file_read(literals: &ByteLiterals, source: &[u8], file: &mut ByteFile) {
    if file.known == 0 {
        file.set = [0; 4];
        for byte in source {
            if let Some(word) = file.set.get_mut(usize::from(*byte / 64)) {
                *word |= 1 << (byte % 64);
            }
        }
        file.held.clear();
    }
    let count = literals.literals.len();
    file.held.resize(count.div_ceil(64), 0);
    for (at, search) in literals.literals.iter().enumerate().skip(file.known) {
        let holds = match search {
            ByteSearch::Byte(byte) => file
                .set
                .get(usize::from(*byte / 64))
                .is_some_and(|word| (word >> (byte % 64)) & 1 == 1),
            ByteSearch::Text(finder) => finder.find(source).is_some(),
        };
        if holds && let Some(word) = file.held.get_mut(at / 64) {
            *word |= 1 << (at % 64);
        }
    }
    file.known = count;
}
