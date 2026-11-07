//! Tabs, wide characters and every line of the candidate count toward the width.
use super::service::fits;
use super::types::{CandidateText, Fit, LinePrefix, LineWidth, TabStop};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
type Input = (&'static str, &'static str, usize, usize);
type Value = Fit;
type Failure = crate::domain::contract::types::PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_tab_prefix_leaves_room_for_one_column"),
        word: Word::Succeeds,
        input: || ("x", "\t", 5, 4),
        expect: Expect::Value(Fit::Within),
    },
    Case {
        name: CaseName("a_tab_prefix_refuses_two_columns"),
        word: Word::Succeeds,
        input: || ("xx", "\t", 5, 4),
        expect: Expect::Value(Fit::Over),
    },
    Case {
        name: CaseName("wide_and_combining_characters_count_their_width"),
        word: Word::Succeeds,
        input: || ("界e\u{301}", "", 3, 4),
        expect: Expect::Value(Fit::Within),
    },
    Case {
        name: CaseName("a_later_line_counts_too"),
        word: Word::Succeeds,
        input: || ("x\n123456", "", 5, 4),
        expect: Expect::Value(Fit::Over),
    },
    Case {
        name: CaseName("a_carriage_return_line_feed_is_one_break"),
        word: Word::Succeeds,
        input: || ("123456\r\nx", "", 6, 4),
        expect: Expect::Value(Fit::Within),
    },
    Case {
        name: CaseName("a_zero_tab_width_is_not_an_operation"),
        word: Word::Refuses,
        input: || ("x", "", 5, 0),
        expect: Expect::Error(crate::domain::contract::types::PortableOperatorFault::WrongOperation),
    },
];
fn subject((text, prefix, maximum, tab): Input) -> Result<Value, Failure> {
    fits(CandidateText(text), LinePrefix(prefix), LineWidth(maximum), TabStop(tab))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
