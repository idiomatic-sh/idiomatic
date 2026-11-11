//! The `format_syntax` service's cases: flag characters fed in order until one is no flag, and the spelling of the flags kept.

use super::service::{flag_text, insert_flag};
use super::types::{FlagRecognition, FormatCharacter, FormatFlags};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The characters after a `%`, fed to the flag grammar in order.
type Input = &'static str;
/// How many characters the grammar consumed as flags, and the canonical spelling of the flags it kept.
type Value = (usize, &'static str);
/// Reading flags is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_flag_consumes_nothing"),
        word: Word::Succeeds,
        input: || "d",
        expect: Expect::Value((0, "")),
    },
    Case {
        name: CaseName("flags_are_spelled_in_ascii_order"),
        word: Word::Succeeds,
        input: || "0-+ #d",
        expect: Expect::Value((5, " #+-0")),
    },
    Case {
        name: CaseName("a_repeated_flag_is_kept_once"),
        word: Word::Succeeds,
        input: || "++-d",
        expect: Expect::Value((3, "+-")),
    },
    Case {
        name: CaseName("the_first_non_flag_stops_the_flags"),
        word: Word::Succeeds,
        input: || "#5-d",
        expect: Expect::Value((1, "#")),
    },
];

/// Feeds the input's characters to the flag grammar until one is no flag, then spells the flags kept.
fn subject(input: Input) -> Result<Value, Failure> {
    let mut flags = FormatFlags::default();
    let mut consumed = 0;
    for character in input.chars() {
        if insert_flag(&mut flags, FormatCharacter(character)) == FlagRecognition::Other {
            break;
        }
        consumed += 1;
    }
    Ok((consumed, flag_text(flags).0))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
