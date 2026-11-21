//! The services of the `verdicts` part: a test's verdicts found by its key, a word's verdict read and kept.

use super::types::{
    Against, TextTestKey, TextVerdicts, VerdictBits, WordBounds, WordKnown, WordPlace, WordVerdicts,
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
                outside: WordKnown::NotYet,
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

/// Where a cell's verdict lies: a cell that is no word of the run nor of the file has no place of its own.
#[inline]
fn place_of(word: CellWord, bounds: WordBounds) -> WordPlace {
    if word < bounds.base {
        WordPlace::Run(Count(usize::try_from(word.0).unwrap_or(usize::MAX)))
    } else if word < bounds.end {
        WordPlace::Own(Count(
            usize::try_from(word.0.wrapping_sub(bounds.base.0)).unwrap_or(usize::MAX),
        ))
    } else {
        WordPlace::Outside
    }
}

/// What two bits say.
#[inline]
const fn said(held: VerdictBits) -> WordKnown {
    match held.0 {
        1 => WordKnown::Fails,
        2 => WordKnown::Holds,
        0 | 3..=u64::MAX => WordKnown::NotYet,
    }
}

/// The two bits of a place among bits; nought past their end.
#[inline]
fn bits_at(bits: &[VerdictBits], at: Count) -> VerdictBits {
    match bits.get(at.0 / WORDS.0) {
        Some(held) => VerdictBits((held.0 >> ((at.0 % WORDS.0) * 2)) & 3),
        None => VerdictBits(0),
    }
}

/// What a test is known to say of a cell.
#[inline]
pub(crate) fn known(test: &WordVerdicts, word: CellWord, bounds: WordBounds) -> WordKnown {
    match place_of(word, bounds) {
        WordPlace::Run(at) => said(bits_at(&test.base, at)),
        WordPlace::Own(at) => said(bits_at(&test.own, at)),
        WordPlace::Outside => test.outside,
    }
}

/// Keeps what a test said of a cell. **The memory bound**: a test's `base` never holds more than two bits for each
/// word of the run (`bounds.base` words), its `own` never more than two bits for each own word the file has interned
/// (`bounds.end - bounds.base` words), and a cell that is neither is kept as the one verdict `outside`. No length here
/// is computed from a cell that was not first found below `bounds.end`.
#[inline]
pub(crate) fn remembered(
    test: &mut WordVerdicts,
    word: CellWord,
    bounds: WordBounds,
    kept: RowKept,
) {
    let known = match kept {
        RowKept::No => WordKnown::Fails,
        RowKept::Yes => WordKnown::Holds,
    };
    let (bits, at) = match place_of(word, bounds) {
        WordPlace::Run(at) => (&mut test.base, at),
        WordPlace::Own(at) => (&mut test.own, at),
        WordPlace::Outside => {
            test.outside = known;
            return;
        }
    };
    let place = at.0 / WORDS.0;
    if bits.len() <= place {
        bits.resize(place.saturating_add(1), VerdictBits(0));
    }
    let two: u64 = match known {
        WordKnown::Fails => 1,
        WordKnown::Holds => 2,
        WordKnown::NotYet => 0,
    };
    if let Some(held) = bits.get_mut(place) {
        held.0 |= two << ((at.0 % WORDS.0) * 2);
    }
}

/// How many words of bits a test keeps, of the run's words and of the file's own: what a case holds to the bound.
#[cfg(test)]
pub(crate) fn kept_bits(test: &WordVerdicts) -> [Count; 2] {
    [Count(test.base.len()), Count(test.own.len())]
}
