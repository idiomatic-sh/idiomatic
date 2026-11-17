//! The run budget's one check of a unit's makings and the one order of makings by fan-out: makings within the rows per row fed
//! or within the floor pass, and so does a text map bounded by its text; makings past both, or an at-most-input operator
//! writing more than it was fed, are over budget; the worst making is the one that wrote the most rows per row fed, of those
//! past the floor.
use super::errors::GrowthError;
use super::service::{against_worst, fed_rows, within_budget};
use super::types::{Fan, Growth, Making};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::types::RowCount;
use std::cmp::Ordering;

/// What a case asks.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Whether makings of an operator that grows so, fed these rows per input and writing these, are within the run budget.
    Budget(Growth, [usize; 3], usize),
    /// The making of these, each its rows fed and written, that the run keeps as its worst, by its place.
    Worst(&'static [(usize, usize)]),
}

/// What a case answered.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// Within the budget.
    Within,
    /// The place of the worst making, none when none wrote the floor.
    Worst(Option<usize>),
}

type Input = Given;
type Value = Answer;
type Failure = GrowthError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_product_within_the_rows_per_row_fed_passes"),
        word: Word::Succeeds,
        input: || Given::Budget(Growth::Keyed(Fan::FileProduct), [300, 300, 0], 90_000),
        expect: Expect::Value(Answer::Within),
    },
    Case {
        name: CaseName("a_making_within_the_floor_passes_whatever_it_was_fed"),
        word: Word::Succeeds,
        input: || Given::Budget(Growth::Keyed(Fan::Repeats), [1, 0, 0], 65_536),
        expect: Expect::Value(Answer::Within),
    },
    Case {
        name: CaseName("a_text_map_past_the_rows_per_row_fed_passes_bounded_by_its_text"),
        word: Word::Succeeds,
        input: || Given::Budget(Growth::PerRowText, [200, 0, 0], 400_000),
        expect: Expect::Value(Answer::Within),
    },
    Case {
        name: CaseName("an_at_most_input_making_writing_what_it_was_fed_passes"),
        word: Word::Succeeds,
        input: || Given::Budget(Growth::AtMostInput, [2, 3, 0], 5),
        expect: Expect::Value(Answer::Within),
    },
    Case {
        name: CaseName("a_making_under_the_floor_is_never_the_worst"),
        word: Word::Succeeds,
        input: || Given::Worst(&[(1, 65_535)]),
        expect: Expect::Value(Answer::Worst(None)),
    },
    Case {
        name: CaseName("the_worst_making_wrote_the_most_rows_per_row_fed"),
        word: Word::Succeeds,
        input: || Given::Worst(&[(1_000, 70_000), (10, 70_000), (100, 900_000)]),
        expect: Expect::Value(Answer::Worst(Some(2))),
    },
    Case {
        name: CaseName("a_making_past_the_rows_per_row_fed_and_the_floor_is_over_budget"),
        word: Word::Refuses,
        input: || Given::Budget(Growth::Keyed(Fan::FileProduct), [600, 600, 0], 360_000),
        expect: Expect::Error(GrowthError::OverBudget {
            fed: RowCount(1_200),
            wrote: RowCount(360_000),
        }),
    },
    Case {
        name: CaseName("an_at_most_input_making_writing_more_than_it_was_fed_is_over_budget"),
        word: Word::Refuses,
        input: || Given::Budget(Growth::AtMostInput, [2, 3, 0], 6),
        expect: Expect::Error(GrowthError::OverBudget {
            fed: RowCount(5),
            wrote: RowCount(6),
        }),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Budget(growth, fed, wrote) => {
            within_budget(growth, fed_rows(&fed.map(RowCount)), RowCount(wrote))
                .map(|()| Answer::Within)
        }
        Given::Worst(makings) => {
            let mut worst: Option<(usize, Making)> = None;
            for (at, (fed, wrote)) in makings.iter().enumerate() {
                let making = Making {
                    fed: RowCount(*fed),
                    wrote: RowCount(*wrote),
                };
                if against_worst(making, worst.map(|(_, kept)| kept)) == Ordering::Greater {
                    worst = Some((at, making));
                }
            }
            Ok(Answer::Worst(worst.map(|(at, _)| at)))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
