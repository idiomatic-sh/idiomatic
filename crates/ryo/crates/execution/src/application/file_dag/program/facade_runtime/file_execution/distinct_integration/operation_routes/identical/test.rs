//! Fused and single-instruction programs publish identical rows in identical order.
use super::super::super::*;
use super::super::fixtures::chain::{Chain, execute_chain};
use super::super::fixtures::describe;
use crate::domain::executable_program::fusion::types::{Barrier, Fusibility};
use ryo_values::domain::query_operators::types::OperationKind;
fn engine() -> Arc<dyn crate::outbound::operators::interface::OperatorsPort> {
    Arc::new(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())))
}
#[test]
fn all26_fused_and_single_instruction_programs_publish_identical_rows_in_identical_order() {
    for &kind in OperationKind::ALL {
        let fused = execute_chain(kind, Chain::Fused, engine());
        let single = execute_chain(kind, Chain::Held, engine());
        assert!(
            !fused.continuations.is_empty(),
            "{kind:?} continues something"
        );
        assert!(
            fused
                .continuations
                .iter()
                .all(|class| *class == Some(Fusibility::Fusable)),
            "{kind:?}: the pure projection fuses into its sole-reader producer"
        );
        assert!(
            single
                .continuations
                .iter()
                .all(|class| *class == Some(Fusibility::Barrier(Barrier::SharedConsumer))),
            "{kind:?}: a second reader keeps the producer's publication"
        );
        assert!(fused.outcome.is_ok(), "{kind:?}: {}", describe(&fused));
        assert_eq!(describe(&fused), describe(&single), "{kind:?}");
    }
}
