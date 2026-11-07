//! The prepared derived-view use a contract names for one kind.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::OperatorContract;

pub(in crate::application::file_dag::invocation) fn prepared(
    contract: &OperatorContract,
    kind: ryo_derived::domain::contract::types::DerivedKind,
) -> Result<&crate::domain::derived_views::types::contract::value::PreparedViewUse, OperatingError> {
    let slots = &contract.derived.slots;
    slots[kind.ordinal()]
        .as_ref()
        .map(|group| group.primary.as_ref())
        .ok_or(OperatingError::NotRun)
}
