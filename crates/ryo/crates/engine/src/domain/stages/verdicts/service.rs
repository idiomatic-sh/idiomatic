//! The services of the `verdicts` part: a test's verdicts found by its key, a word's verdict read and kept.

use super::types::{
    Against, TextTestKey, TextVerdicts, VerdictBits, WordKnown, WordPlace, WordVerdicts,
};
use crate::domain::indexes::reading::service::{folded, hashed};
use crate::domain::indexes::types::{FileStamp, RowHash};
use crate::domain::stages::types::RowKept;
use crate::domain::steps::types::TestOperation;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// How many words' verdicts one word of bits holds.
const WORDS: Count = Count(32);

/// The hash a test's verdicts are found by: the test, its expression, and what it is against.
fn key_hash(key: TextTestKey) -> RowHash {
    let test = match key.test {
        TestOperation::Matches(regex) => [1, u64::from(regex.0)],
        TestOperation::Glob => [2, 0],
        TestOperation::NotGlob => [3, 0],
        TestOperation::StartsWith => [4, 0],
        TestOperation::EndsWith => [5, 0],
        TestOperation::Equal
        | TestOperation::NotEqual
        | TestOperation::Less
        | TestOperation::LessOrEqual
        | TestOperation::Greater
        | TestOperation::GreaterOrEqual
        | TestOperation::In
        | TestOperation::NotIn
        | TestOperation::NumericAtLeast
        | TestOperation::NaturalLess
        | TestOperation::Successor => [0, 0],
    };
    let against = match key.against {
        Against::Nothing => [0, 0],
        Against::Cell(cell) => [1, cell.0],
        Against::Set(place) => [2, u64::from(place.0)],
    };
    let [code, regex] = test;
    let [kind, value] = against;
    folded(hashed(&[
        CellWord(code),
        CellWord(regex),
        CellWord(kind),
        CellWord(value),
    ]))
}

/// The place of a test's verdicts, made empty when the test was not asked before; and, for the file `stamp` names,
/// with the verdicts of another file's own words forgotten: a word of the last file is another text in this one.
pub(crate) fn opened(verdicts: &mut TextVerdicts, key: TextTestKey, stamp: FileStamp) -> Count {
    let TextVerdicts { tests, places } = verdicts;
    let hash = key_hash(key);
    let found = places
        .find(hash.0, |at| {
            tests.get(at.0).is_some_and(|held| held.key == key)
        })
        .copied();
    let at = match found {
        Some(at) => at,
        None => {
            let at = Count(tests.len());
            tests.push(WordVerdicts {
                key,
                base: Vec::new(),
                own: Vec::new(),
                stamp,
                globs: None,
            });
            drop(places.insert_unique(hash.0, at, |other| {
                tests.get(other.0).map_or(0, |held| key_hash(held.key).0)
            }));
            at
        }
    };
    if let Some(held) = tests.get_mut(at.0)
        && held.stamp != stamp
    {
        held.own.clear();
        held.stamp = stamp;
    }
    at
}

/// Where a word's two bits lie; `base` is the file's first own word.
#[inline]
fn place_of(word: CellWord, base: CellWord) -> WordPlace {
    if word < base {
        WordPlace::Run(Count(usize::try_from(word.0).unwrap_or(usize::MAX)))
    } else {
        WordPlace::Own(Count(
            usize::try_from(word.0.wrapping_sub(base.0)).unwrap_or(usize::MAX),
        ))
    }
}

/// What a test is known to say of a word; `base` is the file's first own word.
#[inline]
pub(crate) fn known(test: &WordVerdicts, word: CellWord, base: CellWord) -> WordKnown {
    let (bits, at) = match place_of(word, base) {
        WordPlace::Run(at) => (&test.base, at),
        WordPlace::Own(at) => (&test.own, at),
    };
    let held = match bits.get(at.0 / WORDS.0) {
        Some(held) => (held.0 >> ((at.0 % WORDS.0) * 2)) & 3,
        None => 0,
    };
    match held {
        1 => WordKnown::Fails,
        2 => WordKnown::Holds,
        0 | 3..=u64::MAX => WordKnown::NotYet,
    }
}

/// Keeps what a test said of a word.
#[inline]
pub(crate) fn remembered(test: &mut WordVerdicts, word: CellWord, base: CellWord, kept: RowKept) {
    let (bits, at) = match place_of(word, base) {
        WordPlace::Run(at) => (&mut test.base, at),
        WordPlace::Own(at) => (&mut test.own, at),
    };
    let place = at.0 / WORDS.0;
    if bits.len() <= place {
        bits.resize(place.saturating_add(1), VerdictBits(0));
    }
    let said: u64 = match kept {
        RowKept::No => 1,
        RowKept::Yes => 2,
    };
    if let Some(held) = bits.get_mut(place) {
        held.0 |= said << ((at.0 % WORDS.0) * 2);
    }
}
