//! The prepared derived-view use a contract names for one kind.
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::types::OperatorContract;

pub(crate) fn prepared(
    contract: &OperatorContract,
    kind: ryo_derived::domain::request::types::DerivedKind,
) -> Result<&ryo_execution_domain::domain::derived_views::types::contract::value::PreparedViewUse, OperatingError> {
    let slots = &contract.derived.slots;
    slots[kind.ordinal()]
        .as_ref()
        .map(|group| group.primary.as_ref())
        .ok_or(OperatingError::NotRun)
}
