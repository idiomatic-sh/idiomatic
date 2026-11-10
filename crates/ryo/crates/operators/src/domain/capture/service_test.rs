//! Enumerates UTF-8 character boundaries, nonempty slots and crossing policy.
use super::service::matches;
use super::types::{CaptureSlots, SlotPattern, SlotSubject};
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = (&'static str, &'static str);
type Value = std::borrow::Cow<'static, str>;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("slots_never_split_utf8_characters"),
        word: Word::Succeeds,
        input: || ("{a}{b}", "éab"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("é,ab|éa,b")),
    },
    Case {
        name: CaseName("ordinary_slot_does_not_cross_slashes"),
        word: Word::Succeeds,
        input: || ("{a}.rs", "a/b.rs"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("crossing_slot_keeps_path_separator"),
        word: Word::Succeeds,
        input: || ("{*a}.rs", "a/b.rs"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("a/b")),
    },
    Case {
        name: CaseName("empty_slot_is_not_a_match"),
        word: Word::Succeeds,
        input: || ("{a}.rs", ".rs"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
];
fn subject((pattern, text): Input) -> Result<Value, Failure> {
    let mut slots = Vec::new();
    let mut output: Vec<Vec<String>> = Vec::new();
    matches(
        SlotPattern(pattern),
        SlotSubject(text),
        SlotSubject(text),
        CaptureSlots(&mut slots),
        &mut |slots: &[std::ops::Range<usize>]| {
            output.push(
                slots
                    .iter()
                    .map(|range| text[range.clone()].to_owned())
                    .collect(),
            )
        },
    );
    assert!(slots.is_empty());
    Ok(std::borrow::Cow::Owned(
        output
            .iter()
            .map(|slots| slots.join(","))
            .collect::<Vec<_>>()
            .join("|"),
    ))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
