//! The operators-port witness and refusal the capability proofs share with the join proofs beside them.
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_values::domain::rows::types::Table;
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
            self.calls.fetch_add(1, Ordering::Relaxed);
            if self.refuse {
                return Err(PortableOperatorFault::NoSuchColumn {
                    index: ryo_values::domain::plan_parts::types::ColumnIndex(REFUSAL),
                });
            }
        }
        crate::application::file_dag::fixtures::operators().operators(call)
    }
}
pub(crate) fn refusal(result: Result<Table, OperatingError>) {
    assert!(
        matches!(
            result,
            Err(OperatingError::Table(
                ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: REFUSAL }
            ))
        ),
        "{result:?}"
    );
}
