//! Empty semantic input must clear reused scratch without consulting row callbacks.
use super::{
    service::flag,
    types::{OrderedExistenceDriver, OrderedExistenceScratch},
};
use crate::domain::equality_join::errors::EqualityJoinError;
use crate::domain::equality_join::types::PartitionExtremum;
use crate::domain::ordered_existence::types::ExtremumWitness;
use ryo_relations::domain::relations::types::{Integer, Value as ValueCell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

struct Empty;
impl OrderedExistenceDriver for Empty {
    fn len(&self) -> RowCount {
        RowCount(0)
    }
    fn key_cell(&mut self, _: RowIndex, _: ColumnIndex) -> Result<ValueCell, EqualityJoinError> {
        panic!("empty input cannot read a key")
    }
    fn number(&mut self, _: RowIndex, _: ColumnIndex) -> Result<Integer, EqualityJoinError> {
        panic!("empty input cannot read an ordering value")
    }
    fn emit_dominated(&mut self, _: RowIndex, _: RowIndex) -> Result<(), EqualityJoinError> {
        panic!("empty input cannot emit a row")
    }
}

use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = PartitionExtremum;
type Value = ();
type Failure = crate::domain::contract::types::PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("minimum_clears_prior_state"),
        word: Word::Succeeds,
        input: || PartitionExtremum::Minimum,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("maximum_clears_prior_state"),
        word: Word::Succeeds,
        input: || PartitionExtremum::Maximum,
        expect: Expect::Value(()),
    },
];
fn subject(extremum: Input) -> Result<Value, Failure> {
    let mut scratch = OrderedExistenceScratch::default();
    scratch.key.push(ValueCell::Number(Integer(9)));
    scratch.groups.insert(
        vec![ValueCell::Number(Integer(9))],
        ExtremumWitness {
            value: Integer(3),
            row: RowIndex(8),
        },
    );
    flag(
        &[ColumnIndex(0)],
        ColumnIndex(1),
        extremum,
        &mut Empty,
        &mut scratch,
    )?;
    assert!(scratch.groups.is_empty());
    assert!(scratch.key.is_empty());
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
