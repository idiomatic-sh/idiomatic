//! The operators fakes the execution use case's `shared_walk` boundary scenarios inject.
use crate::outbound::operators::interface::OperatorsPort as _;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub struct SemanticCalls(pub AtomicUsize);

impl crate::outbound::operators::interface::OperatorsPort for SemanticCalls {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if matches!(
            &call,
            OperatorCall::Evaluate {
                operation: ryo_values::domain::plans::types::Operation::Select { .. },
                ..
            }
        ) {
            let _previous = self.0.fetch_add(1, Ordering::Relaxed);
        }
        OperatorsUseCaseImpl::new(std::sync::Arc::new(
            ryo_operators::outbound::matching::factory::use_case(),
        ))
        .operators(call)
    }
}

impl SemanticCalls {
    pub fn selects(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}
