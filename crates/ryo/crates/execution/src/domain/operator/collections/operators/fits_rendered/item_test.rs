//! The `fits_rendered` item's cases.

use super::super::fixtures::contract::check;
use super::super::interface::OperatorsItem;
use super::item::FitsRenderedItem;
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::QueryOperator;

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

/// What a row asks: to measure a candidate, to describe the operator, or to prepare an operation that is not its own.
#[derive(Clone, Copy)]
enum Asked {
    /// Whether the candidate fits.
    Measure(Candidate),
    /// The shared cases of the collection.
    Describe,
    /// Preparing a union with this item.
    Foreign,
}

type Input = Asked;
type Value = bool;
type Failure = OperatingError;

fn subject(asked: Input) -> Result<Value, Failure> {
    let candidate = match asked {
        Asked::Measure(candidate) => candidate,
        Asked::Describe => {
            check(QueryOperator::FitsRendered);
            return Ok(true);
        }
        Asked::Foreign => {
            return FitsRenderedItem
                .prepare(
                    &Operation::Union {
                        left: Source::Relation(Relation::Function),
                        right: Source::Relation(Relation::Function),
                    },
                    &[],
                    &[],
                )
                .map(|_| true);
        }
    };
    use ryo_operators::domain::fits_rendered::types::{CandidateText, LinePrefix, LineWidth, TabStop};
    use ryo_operators::inbound::use_cases::operators::{
        interface::OperatorsUseCase,
        types::{OperatorAnswer, OperatorCall},
        use_case::OperatorsUseCaseImpl,
    };
    let operators = OperatorsUseCaseImpl::new(std::sync::Arc::new(
        ryo_operators::outbound::matching::factory::use_case(),
    ));
    let answer = operators
        .operators(OperatorCall::FitsRendered {
            rendered: CandidateText(candidate.text),
            prefix: LinePrefix(candidate.prefix),
            maximum: LineWidth(candidate.maximum),
            tab_width: TabStop(candidate.tab_width),
        })
        .map_err(crate::domain::operator::contract::types::portable_fault)?;
    let OperatorAnswer::Selected(selection) = answer else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(selection == ryo_operators::domain::select::types::RowSelection::Kept)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_tab_in_the_prefix_advances_to_the_next_stop"),
        word: Word::Succeeds,
        input: || Asked::Measure(Candidate {
            text: "x",
            prefix: "\t",
            maximum: 5,
            tab_width: 4,
        }),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_candidate_past_the_widest_line_does_not_fit"),
        word: Word::Succeeds,
        input: || Asked::Measure(Candidate {
            text: "xx",
            prefix: "\t",
            maximum: 5,
            tab_width: 4,
        }),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_wide_character_and_a_combining_mark_count_by_display_width"),
        word: Word::Succeeds,
        input: || Asked::Measure(Candidate {
            text: "界e\u{301}",
            prefix: "",
            maximum: 3,
            tab_width: 4,
        }),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("every_line_of_the_candidate_is_measured"),
        word: Word::Succeeds,
        input: || Asked::Measure(Candidate {
            text: "x\n123456",
            prefix: "",
            maximum: 5,
            tab_width: 4,
        }),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_carriage_return_and_line_feed_end_one_line"),
        word: Word::Succeeds,
        input: || Asked::Measure(Candidate {
            text: "123456\r\nx",
            prefix: "",
            maximum: 6,
            tab_width: 4,
        }),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_zero_tab_width_is_not_an_operation"),
        word: Word::Refuses,
        input: || Asked::Measure(Candidate {
            text: "x",
            prefix: "\t",
            maximum: 5,
            tab_width: 0,
        }),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_operator_describes_itself"),
        word: Word::Succeeds,
        input: || Asked::Describe,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_operation_that_is_not_its_own_is_refused"),
        word: Word::Refuses,
        input: || Asked::Foreign,
        expect: Expect::Error(OperatingError::WrongOperation),
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
