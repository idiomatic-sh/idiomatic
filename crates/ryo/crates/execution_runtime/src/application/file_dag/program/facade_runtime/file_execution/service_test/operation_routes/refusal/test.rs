//! A production route cannot bypass an injected semantic refusal, and fused programs refuse alike.
use super::super::super::*;
use super::super::fixtures::chain::{Chain, execute_chain, execute_fixture};
use super::super::fixtures::describe;
use ryo_values::domain::query_operators::types::OperationKind;
#[derive(Debug)]
struct RefusingSemantics;
impl crate::outbound::operators::interface::OperatorsPort for RefusingSemantics {
    fn operators(
        &self,
        _: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        Err(ryo_operators::domain::contract::types::PortableOperatorFault::WrongOperation)
    }
}
#[test]
fn production_route_cannot_bypass_an_injected_semantic_refusal() {
    assert!(matches!(
        execute_fixture(OperationKind::Project, false, Arc::new(RefusingSemantics)),
        Err(OperatingError::WrongOperation)
    ));
}
#[test]
fn all26_fused_and_single_instruction_programs_refuse_with_the_same_error() {
    for &kind in OperationKind::ALL {
        let fused = execute_chain(kind, Chain::Fused, Arc::new(RefusingSemantics));
        let single = execute_chain(kind, Chain::Held, Arc::new(RefusingSemantics));
        assert_eq!(describe(&fused), describe(&single), "{kind:?}");
    }
}
