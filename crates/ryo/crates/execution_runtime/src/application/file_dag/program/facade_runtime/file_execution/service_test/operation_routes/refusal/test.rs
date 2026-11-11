//! A production route cannot bypass an injected semantic refusal, and fused programs refuse alike.
use super::super::super::super::*;
use super::super::fixtures::chain::{Chain, execute_chain, execute_fixture};
use super::super::fixtures::describe;
use super::fixtures::RefusingSemantics;
use ryo_values::domain::query_operators::types::OperationKind;
use std::sync::Arc;
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
