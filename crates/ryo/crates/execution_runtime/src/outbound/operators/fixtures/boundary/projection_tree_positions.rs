//! The operators fake the execution use case's tree-position projection scenarios inject.
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase as _;
use ryo_operators::inbound::use_cases::operators::types::{OperatorAnswer, OperatorCall};
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
/// A join call counter that can refuse every join it sees, standing in for the operators port.
pub struct ObservedJoin {
    pub calls: AtomicUsize,
    pub refuse: bool,
}
impl crate::outbound::operators::interface::OperatorsPort for ObservedJoin {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if matches!(
            &call,
            OperatorCall::Evaluate {
                operation: ryo_values::domain::plans::types::Operation::Join { .. },
                ..
            }
        ) {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if self.refuse {
                return Err(PortableOperatorFault::ShapeMismatch);
            }
        }
        OperatorsUseCaseImpl::new(std::sync::Arc::new(
            ryo_operators::outbound::matching::factory::use_case(),
        ))
        .operators(call)
    }
}
