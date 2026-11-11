//! The operators-port witness and refusal the capability proofs share with the join proofs beside them.
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use std::sync::atomic::{AtomicUsize, Ordering};
pub(crate) const REFUSAL: u32 = 60001;
#[derive(Debug, Default)]
pub(crate) struct Witness {
    pub(crate) calls: AtomicUsize,
    pub(crate) refuse: bool,
}
impl crate::outbound::operators::interface::OperatorsPort for Witness {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if let OperatorCall::Evaluate {
            operation: ryo_values::domain::plans::types::Operation::Join { .. },
            ..
        } = &call
        {
            let _before = self.calls.fetch_add(1, Ordering::Relaxed);
            if self.refuse {
                return Err(PortableOperatorFault::NoSuchColumn {
                    index: ryo_values::domain::plan_parts::types::ColumnIndex(REFUSAL),
                });
            }
        }
        crate::application::file_dag::fixtures::operators().operators(call)
    }
}
