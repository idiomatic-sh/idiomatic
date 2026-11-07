//! The prepared derived-view use a contract names for one kind.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::OperatorContract;

pub(in crate::application::file_dag::invocation) fn prepared(
    contract: &OperatorContract,
    kind: ryo_derived::domain::request::types::DerivedKind,
) -> Result<&crate::domain::derived_views::types::PreparedViewUse, OperatingError> {
    let slots = &contract.derived.slots;
    slots[crate::domain::derived_views::service::slot(kind)]
        .as_ref()
        .map(|group| group.primary.as_ref())
        .ok_or(OperatingError::NotRun)
}
