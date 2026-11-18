//! A filter of an antijoin is made on its left, which the antijoin keeps as it is, and on its right too when it tests only keys,
//! which a right row that removes a left row holds the same values in; a comparison of no key is never made on the right. What
//! every filter of the antijoin shares is made below it once, and each filter keeps what is its own.

use super::super::service::push_selections;
use super::{at, named, relation, same, select, step, tested};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as Reads, StepId};
use ryo_values::domain::plans::types::{Operation, PlanStep};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A filter of an antijoin keyed on the edge tests two of its keys equal, then a column that is no key.
    FilterCrossesAntijoin,
    /// A filter of an antijoin keyed on the edge tests a column that is no key.
    NonKeyStaysOffTheAntijoinsRight,
    /// Two filters of an antijoin keyed on the edge test two of its keys equal, and the first a column that is no key.
    SharedComparisonPlacedOnce,
}

/// Whether the plan came back as the row expects it.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_filter_of_an_antijoin_is_made_on_its_left_and_its_keys_comparisons_on_its_right",
        ),
        word: Word::Succeeds,
        input: || Scenario::FilterCrossesAntijoin,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_comparison_of_no_key_is_never_made_on_the_antijoins_right"),
        word: Word::Succeeds,
        input: || Scenario::NonKeyStaysOffTheAntijoinsRight,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_comparison_every_filter_of_an_antijoin_shares_is_made_below_it_once"),
        word: Word::Succeeds,
        input: || Scenario::SharedComparisonPlacedOnce,
        expect: Expect::Value(true),
    },
];

/// The semantic edges of `left` not among those of `right`, the two matched on their parent, their child and their file.
fn edges_antijoin(left: Reads, right: Reads) -> PlanStep {
    let edge = || vec![ColumnIndex(0), ColumnIndex(1), ColumnIndex(4)];
    step(Operation::Antijoin {
        left,
        right,
        left_on: edge(),
        right_on: edge(),
    })
}

/// The plan a row gives, the step it emits, the plan it expects back, and the step that emits there.
type Planned = (Vec<PlanStep>, StepId, Vec<PlanStep>, StepId);

fn placed(scenario: Scenario) -> Planned {
    match scenario {
        Scenario::FilterCrossesAntijoin => (
            vec![
                edges_antijoin(
                    relation(Relation::SemanticChild),
                    relation(Relation::SemanticChild),
                ),
                select(at(0), vec![same(0, 1), named(3, "body")]),
            ],
            StepId(1),
            vec![
                select(
                    relation(Relation::SemanticChild),
                    vec![same(0, 1), named(3, "body")],
                ),
                select(relation(Relation::SemanticChild), vec![same(0, 1)]),
                edges_antijoin(at(0), at(1)),
            ],
            StepId(2),
        ),
        Scenario::NonKeyStaysOffTheAntijoinsRight => (
            vec![
                edges_antijoin(
                    relation(Relation::SemanticChild),
                    relation(Relation::SemanticChild),
                ),
                select(at(0), vec![named(3, "body")]),
            ],
            StepId(1),
            vec![
                select(relation(Relation::SemanticChild), vec![named(3, "body")]),
                edges_antijoin(at(0), relation(Relation::SemanticChild)),
            ],
            StepId(1),
        ),
        Scenario::SharedComparisonPlacedOnce => (
            vec![
                edges_antijoin(
                    relation(Relation::SemanticChild),
                    relation(Relation::SemanticChild),
                ),
                select(at(0), vec![same(0, 1), named(3, "body")]),
                select(at(0), vec![same(0, 1)]),
                step(Operation::Union {
                    left: at(1),
                    right: at(2),
                }),
            ],
            StepId(3),
            vec![
                // The left's filter and the right's are the same, made once.
                select(relation(Relation::SemanticChild), vec![same(0, 1)]),
                edges_antijoin(at(0), at(0)),
                select(at(1), vec![named(3, "body")]),
                step(Operation::Union {
                    left: at(2),
                    right: at(1),
                }),
            ],
            StepId(3),
        ),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(tested(placed(scenario), push_selections))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
