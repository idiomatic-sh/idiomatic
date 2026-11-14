//! Proves the `GraphLookups` value's behaviour over an invocation that acquired no ordered keys and no
//! adjacency.

use super::super::row_delivery::value::RowDelivery;
use super::value::GraphLookups;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::InputPlace;
use crate::domain::operator::contract::primitive::types::PreparedInputs;
use crate::domain::operator::contract::types::PrimitiveWorkspace;
use crate::domain::operator::fixtures::{Kept, Resources, extend, with_lease};
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// What a case asks of the lookups of a row-mode evaluation.
type Input = for<'f> fn(&mut GraphLookups<'f>, &mut RowDelivery<'_, 'f>) -> Result<RowCount, ()>;
/// How many left rows the ordered keys pair.
type Value = RowCount;
/// The engine error the delivery kept.
type Failure = Option<OperatingError>;

fn subject(case: Input) -> Result<Value, Failure> {
    let resources = Resources::default();
    let mut workspace = PrimitiveWorkspace::default();
    let operation = extend();
    let mut sink = Kept::default();
    with_lease(&resources, PredicateOutcome::Holds, &mut workspace, |lease| {
        let mut lookups = GraphLookups::default();
        let inputs = PreparedInputs::Row {
            input: InputPlace(0),
            row: &[],
            sides: &[],
        };
        let mut rows = RowDelivery::new(lease, &mut sink, &operation, inputs);
        case(&mut lookups, &mut rows).map_err(|()| rows.take_failure())
    })
}

/// The rows of this value's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_left_row_is_paired_before_the_keys_are_acquired"),
        word: Word::Succeeds,
        input: || |lookups, _rows| Ok(lookups.left_len()),
        expect: Expect::Value(RowCount(0)),
    },
    Case {
        name: CaseName("cycles_need_an_adjacency"),
        word: Word::Refuses,
        input: || {
            |lookups, rows| {
                let _adjacency = lookups.cycles(rows).map_err(|_| ())?;
                Ok(RowCount(0))
            }
        },
        expect: Expect::Error(Some(OperatingError::WrongBinding)),
    },
    Case {
        name: CaseName("an_ordered_match_is_no_row_evaluation"),
        word: Word::Refuses,
        input: || {
            |lookups, rows| {
                lookups.ordered_match(rows).map_err(|_| ())?;
                Ok(lookups.left_len())
            }
        },
        expect: Expect::Error(Some(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("a_pair_needs_the_ordered_keys"),
        word: Word::Refuses,
        input: || {
            |lookups, rows| {
                lookups
                    .visit_pair(rows, RowIndex(0), RowIndex(0), &mut |_, _| Ok(()))
                    .map_err(|_| ())?;
                Ok(lookups.left_len())
            }
        },
        expect: Expect::Error(Some(OperatingError::WrongBinding)),
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
