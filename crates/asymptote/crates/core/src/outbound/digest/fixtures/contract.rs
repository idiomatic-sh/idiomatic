//! What every `digest` adapter must pass: whitespace never moves a line's fingerprint, its text does, and parts split
//! differently come to different digests.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::violations::types::SourceText;
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::DigestParts;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// Two texts: is line one's fingerprint the same?
    Lines(&'static str, &'static str),
    /// Two lists of parts: is their digest the same?
    Parts(&'static [&'static [u8]], &'static [&'static [u8]]),
}

/// The two texts, or the two lists of parts, a row compares.
type Input = Question;
/// Whether they agree.
type Value = bool;
/// Digesting cannot fail, so no row has an error to expect.
type Failure = ();

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reindenting_a_line_keeps_its_fingerprint"),
        word: Word::Succeeds,
        input: || Question::Lines("fn a() {}\n", "fn  a()\t{}\r\n"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("changing_a_lines_text_changes_it"),
        word: Word::Succeeds,
        input: || Question::Lines("fn a() {}\n", "fn b() {}\n"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("editing_another_line_keeps_it"),
        word: Word::Succeeds,
        input: || Question::Lines("fn a() {}\nlet x = 1;\n", "fn a() {}\nlet y = 2;\n"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("parts_split_differently_come_to_different_digests"),
        word: Word::Succeeds,
        input: || Question::Parts(&[b"ab"], &[b"a", b"b"]),
        expect: Expect::Value(false),
    },
];

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl DigestPort) {
    run(
        CASES,
        Word::Succeeds,
        |question| -> Result<Value, Failure> {
            Ok(match question {
                Question::Lines(one, other) => {
                    let first =
                        |text: &str| adapter.lines(&SourceText(text.into())).0.first().copied();
                    first(one) == first(other)
                }
                Question::Parts(one, other) => {
                    adapter.digest(DigestParts(one)) == adapter.digest(DigestParts(other))
                }
            })
        },
    );
}
