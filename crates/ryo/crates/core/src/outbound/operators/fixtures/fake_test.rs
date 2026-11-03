//! The fake and the production adapter obey the same port contract.
use super::{contract, fake::OperatorsFake};
use crate::outbound::operators::{
    adapters::use_case::adapter::OperatorsUseCaseAdapter, interface::OperatorsPort,
};
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_operators::domain::contract::types::{OperatorCall, PortableOperatorFault};
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::atomic::Ordering;

#[test]
fn both_adapters_preserve_the_port_contract() {
    contract::check(&OperatorsFake::default());
    contract::check(&OperatorsUseCaseAdapter::new(OperatorsUseCaseImpl::new()));
}

#[test]
fn controlled_refusal_precedes_mutation_and_records_one_call() {
    let fake = OperatorsFake {
        refusal: Some(PortableOperatorFault::NoSuchColumn { index: 8 }),
        ..OperatorsFake::default()
    };
    let row = [Value::Number(Integer(3))];
    let mut target = vec![Value::Number(Integer(9))];
    let answer = fake.operators(OperatorCall::Project {
        row: &row,
        columns: &[ColumnIndex(0)],
        target: &mut target,
    });
    assert_eq!(
        answer,
        Err(PortableOperatorFault::NoSuchColumn { index: 8 })
    );
    assert_eq!(target, vec![Value::Number(Integer(9))]);
    assert_eq!(fake.calls.load(Ordering::Relaxed), 1);
}
