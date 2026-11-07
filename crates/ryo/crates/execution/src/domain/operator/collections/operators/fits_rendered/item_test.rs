//! The `fits_rendered` item's cases.

use super::item::fits;
use crate::domain::operating::errors::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// A rendered candidate, the prefix before it, the widest line allowed and the tab width.
#[derive(Clone, Copy)]
struct Candidate {
    /// The rendered text.
    text: &'static str,
    /// The text before it on its line.
    prefix: &'static str,
    /// The widest line allowed.
    maximum: usize,
    /// The columns a tab advances to the next multiple of.
    tab_width: usize,
}

type Input = Candidate;
type Value = bool;
type Failure = OperatingError;

fn subject(candidate: Input) -> Result<Value, Failure> {
    fits(
        candidate.text,
        candidate.prefix,
        candidate.maximum,
        candidate.tab_width,
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_tab_in_the_prefix_advances_to_the_next_stop"),
        word: Word::Succeeds,
        input: || Candidate {
            text: "x",
            prefix: "\t",
            maximum: 5,
            tab_width: 4,
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_candidate_past_the_widest_line_does_not_fit"),
        word: Word::Succeeds,
        input: || Candidate {
            text: "xx",
            prefix: "\t",
            maximum: 5,
            tab_width: 4,
        },
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_wide_character_and_a_combining_mark_count_by_display_width"),
        word: Word::Succeeds,
        input: || Candidate {
            text: "界e\u{301}",
            prefix: "",
            maximum: 3,
            tab_width: 4,
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("every_line_of_the_candidate_is_measured"),
        word: Word::Succeeds,
        input: || Candidate {
            text: "x\n123456",
            prefix: "",
            maximum: 5,
            tab_width: 4,
        },
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_carriage_return_and_line_feed_end_one_line"),
        word: Word::Succeeds,
        input: || Candidate {
            text: "123456\r\nx",
            prefix: "",
            maximum: 6,
            tab_width: 4,
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_zero_tab_width_is_an_invalid_span"),
        word: Word::Refuses,
        input: || Candidate {
            text: "x",
            prefix: "\t",
            maximum: 5,
            tab_width: 0,
        },
        expect: Expect::Error(OperatingError::InvalidSpan),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
