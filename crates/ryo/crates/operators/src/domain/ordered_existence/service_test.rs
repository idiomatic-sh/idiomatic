//! Empty semantic input must clear reused scratch without consulting row callbacks.
use super::{
    ColumnIndex, OrderedExistenceDriver, OrderedExistenceScratch, PartitionExtremum, flag,
};
use crate::domain::equality_join::types::JoinInterrupted;
use crate::domain::ordered_existence::types::ExtremumWitness;
use ryo_relations::domain::relations::types::{Integer, Value};

struct Empty;
impl OrderedExistenceDriver for Empty {
    fn len(&self) -> usize {
        0
    }
    fn key_cell(&mut self, _: usize, _: ColumnIndex) -> Result<Value, JoinInterrupted> {
        panic!("empty input cannot read a key")
    }
    fn number(&mut self, _: usize, _: ColumnIndex) -> Result<Integer, JoinInterrupted> {
        panic!("empty input cannot read an ordering value")
    }
    fn emit_dominated(&mut self, _: usize, _: usize) -> Result<(), JoinInterrupted> {
        panic!("empty input cannot emit a row")
    }
}

#[test]
fn empty_input_clears_prior_partition_state_for_both_orders() {
    let mut scratch = OrderedExistenceScratch::default();
    for extremum in [PartitionExtremum::Minimum, PartitionExtremum::Maximum] {
        scratch.key.push(Value::Number(Integer(9)));
        scratch.groups.insert(
            vec![Value::Number(Integer(9))],
            ExtremumWitness {
                value: Integer(3),
                row: 8,
            },
        );
        assert!(
            flag(
                &[ColumnIndex(0)],
                ColumnIndex(1),
                extremum,
                &mut Empty,
                &mut scratch
            )
            .is_ok()
        );
        assert!(scratch.groups.is_empty());
        assert!(scratch.key.is_empty());
    }
}
