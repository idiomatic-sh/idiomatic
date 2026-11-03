//! Controlled stand-in for the operator port; records and can refuse calls.
use super::super::interface::OperatorsPort;
use ryo_operators::domain::contract::types::{OperatorAnswer, OperatorCall, PortableOperatorFault};
use ryo_operators::inbound::use_cases::operators::{
    interface::OperatorsUseCase, use_case::OperatorsUseCaseImpl,
};
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
            return Err(PortableOperatorFault::NoSuchColumn { index: 8 });
        }
        OperatorsUseCaseImpl::new().operators(call)
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
