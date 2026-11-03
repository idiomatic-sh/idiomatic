//! The composition adapter executes the same semantic port contract as the fake.
use super::adapter::OperatorsUseCaseAdapter;
use crate::outbound::operators::fixtures::contract;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;

#[test]
fn injected_use_case_preserves_the_operator_port_contract() {
    contract::check(&OperatorsUseCaseAdapter::new(OperatorsUseCaseImpl::new()));
}
