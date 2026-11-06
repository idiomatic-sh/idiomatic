//! The operators-port witness and refusal the capability proofs share with the join proofs beside them.
use super::super::*;
use ryo_operators::inbound::use_cases::operators::interface::{
    JoinInput, JoinRow, OperatorAnswer, OperatorCall, OperatorsUseCase, PortableOperatorFault,
};
use std::sync::atomic::{AtomicUsize, Ordering};
pub(in super::super) const REFUSAL: u32 = 60001;
#[derive(Debug, Default)]
pub(in super::super) struct Witness {
    pub(in super::super) calls: AtomicUsize,
    pub(in super::super) columns: AtomicUsize,
    pub(in super::super) positions: AtomicUsize,
    pub(in super::super) refuse: bool,
    pub(in super::super) only_columns: bool,
}
impl OperatorsUseCase for Witness {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if let OperatorCall::EqualityJoin { input } = &call {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let columnar = matches!(
                input,
                JoinInput::Values {
                    left: JoinRow::Columns(_),
                    ..
                }
            );
            match input {
                JoinInput::Values {
                    left: JoinRow::Columns(_),
                    ..
                } => {
                    self.columns.fetch_add(1, Ordering::Relaxed);
                }
                JoinInput::Positions { .. } => {
                    self.positions.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
            if self.refuse && (!self.only_columns || columnar) {
                return Err(PortableOperatorFault::NoSuchColumn { index: REFUSAL });
            }
        }
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new()
            .operators(call)
    }
}
pub(crate) fn refusal(result: Result<Table, OperatingError>) {
    assert!(
        matches!(
            result,
            Err(OperatingError::Table(
                ryo_values::domain::rows::errors::TableModelError::NoSuchColumn { index: REFUSAL }
            ))
        ),
        "{result:?}"
    );
}
