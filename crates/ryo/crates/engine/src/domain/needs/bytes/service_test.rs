//! The `bytes` part's cases: a file's bytes read once for the literals the byte terms ask, a single byte from the file's
//! byte set and a longer literal searched, a term of alternatives holding when one is there, a term that asks nothing
//! holding in every file, and a literal met after the file's first read answered by a second.

use super::service::{held, term_wants};
use super::types::{ByteFile, ByteLiterals};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::WordSlot;

/// A file's bytes, and the alternatives of two byte terms (none: the term asks nothing).
type Input = (&'static str, [Option<&'static [&'static str]>; 2]);

/// Whether each term holds in the file.
type Value = [bool; 2];

const CASES: &[Case<Input, Value, ()>] = &[
    Case {
        name: CaseName("a_byte_and_a_longer_literal_the_file_holds_hold"),
        word: Word::Succeeds,
        input: || ("#!/usr/bin/env python\nx = 1\n", [Some(&["#!"]), Some(&["\n"])]),
        expect: Expect::Value([true, true]),
    },
    Case {
        name: CaseName("a_literal_the_file_lacks_is_empty"),
        word: Word::Succeeds,
        input: || ("x = 1\n", [Some(&["#!"]), Some(&["\\"])]),
        expect: Expect::Value([false, false]),
    },
    Case {
        name: CaseName("a_term_of_alternatives_holds_when_one_is_there"),
        word: Word::Succeeds,
        input: || ("a = b | c\n", [Some(&["\u{7}", "|"]), Some(&["\u{7}", "\u{1b}"])]),
        expect: Expect::Value([true, false]),
    },
    Case {
        name: CaseName("a_term_that_asks_nothing_holds_in_every_file"),
        word: Word::Succeeds,
        input: || ("", [None, Some(&[""])]),
        expect: Expect::Value([true, true]),
    },
];

fn subject((source, asked): Input) -> Result<Value, ()> {
    let mut literals = ByteLiterals::default();
    let mut file = ByteFile::default();
    let first = term_wants(&mut literals, (WordSlot(0), WordSlot(0)), || {
        asked[0].map(|texts| texts.iter().map(|text| text.as_bytes().to_vec()).collect())
    });
    // The file is read for the first term's literals before the second term is met.
    let first_held = held(&literals, source.as_bytes(), &mut file, first);
    let second = term_wants(&mut literals, (WordSlot(1), WordSlot(0)), || {
        asked[1].map(|texts| texts.iter().map(|text| text.as_bytes().to_vec()).collect())
    });
    Ok([first_held, held(&literals, source.as_bytes(), &mut file, second)])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
