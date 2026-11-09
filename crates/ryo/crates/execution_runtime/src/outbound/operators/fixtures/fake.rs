//! Controlled stand-in for the operator port; records and can refuse calls.
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase as _;
use ryo_operators::inbound::use_cases::operators::types::{OperatorAnswer, OperatorCall};
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub struct OperatorsFake {
    /// Number of semantic calls made by the engine.
    pub calls: AtomicUsize,
    /// Refuse before the portable implementation runs.
    pub refuses: bool,
}
impl OperatorsPort for OperatorsFake {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.refuses {
            return Err(PortableOperatorFault::NoSuchColumn {
                index: ryo_values::domain::plan_parts::types::ColumnIndex(8),
            });
        }
        OperatorsUseCaseImpl::new(std::sync::Arc::new(
            ryo_operators::outbound::matching::factory::use_case(),
        ))
        .operators(call)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
