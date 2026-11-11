//! Proves the `JoinCandidates` value's behaviour: with neither a native lookup nor an index bound, its right
//! cells are logical and every port that needs candidates keeps the engine's refusal.

use super::super::row_delivery::value::RowDelivery;
use super::value::JoinCandidates;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::primitive::types::PreparedInputs;
use crate::domain::operator::contract::types::PrimitiveWorkspace;
use crate::domain::operator::fixtures::{Kept, Resources, extend, with_lease};
use crate::domain::table_model::types::Verdict;
use ryo_operators::domain::contract::types::RightColumns;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a case asks of the candidates of an evaluation with nothing bound.
type Input = fn(&mut JoinCandidates<'_, '_>, &mut RowDelivery<'_, '_>) -> Result<RightColumns, ()>;
/// How the right cells are addressed.
type Value = RightColumns;
/// The engine error the delivery kept.
type Failure = Option<OperatingError>;

fn subject(case: Input) -> Result<Value, Failure> {
    let resources = Resources::default();
    let mut workspace = PrimitiveWorkspace::default();
    let operation = extend();
    let mut sink = Kept::default();
    with_lease(&resources, Verdict::Holds, &mut workspace, |lease| {
        let mut candidates = JoinCandidates::new(&resources, &operation, &[], Vec::new());
        let mut rows = RowDelivery::new(lease, &mut sink, &operation, PreparedInputs::Views(&[]));
        case(&mut candidates, &mut rows).map_err(|()| rows.take_failure())
    })
}

fn key() -> [Cell; 1] {
    [Cell::Number(Integer(1))]
}

/// The rows of this value's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("without_a_native_lookup_the_right_cells_are_logical"),
        word: Word::Succeeds,
        input: || |candidates, _rows| Ok(candidates.right_columns()),
        expect: Expect::Value(RightColumns::Logical),
    },
    Case {
        name: CaseName("a_join_with_no_index_has_not_run"),
        word: Word::Refuses,
        input: || {
            |candidates, rows| {
                let _advance = candidates.next(rows, &key()).map_err(|_| ())?;
                Ok(candidates.right_columns())
            }
        },
        expect: Expect::Error(Some(OperatingError::NotRun)),
    },
    Case {
        name: CaseName("an_empty_right_needs_its_index"),
        word: Word::Refuses,
        input: || {
            |candidates, rows| {
                let _population = candidates.right_empty(rows).map_err(|_| ())?;
                Ok(candidates.right_columns())
            }
        },
        expect: Expect::Error(Some(OperatingError::WrongBinding)),
    },
    Case {
        name: CaseName("membership_of_the_right_needs_its_index"),
        word: Word::Refuses,
        input: || {
            |candidates, rows| {
                let _membership = candidates.contains_right(rows, &key()).map_err(|_| ())?;
                Ok(candidates.right_columns())
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
