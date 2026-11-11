//! The support of the `join_result` cases.
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;

/// What a row gives: the kernel's terminal answer and the engine error recorded before it.
pub(super) struct Terminal {
    /// The kernel's terminal answer.
    pub(super) answer: Result<OperatorAnswer, PortableOperatorFault>,
    /// The engine error recorded before the answer.
    pub(super) recorded: Option<OperatingError>,
}
