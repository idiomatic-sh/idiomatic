//! The types of the `registration` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::query_operators::types::OperationKind;

/// Only this registry constructs a callable operator selected by the exhaustive kind: no caller outside the
/// part can build one.
///
/// ```compile_fail
/// let _forged = ryo_execution_domain::domain::operator::contract::registration::types::RegisteredOperator {
///     kind: ryo_values::domain::query_operators::types::OperationKind::Select,
/// };
/// ```
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::registration::types::RegisteredOperator;
/// fn named(_: Option<&RegisteredOperator>) {}
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RegisteredOperator {
    pub(super) kind: OperationKind,
}
