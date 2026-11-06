//! Static request interpretation and immutable access; no acquisition or retained policy.
use super::types::{DerivedContract, ViewRequest};
use crate::domain::operating::errors::OperatingError;

pub(crate) fn validate(
    expected: DerivedKind,
    contract: &DerivedContract,
) -> Result<(), OperatingError> {
    if contract.kind != expected || contract.request.kind() != expected {
        return Err(OperatingError::WrongOperation);
    }
    contract.metadata.clone()
}
/// Constant prepared-slot lookup; runtime hits never scan or clone request metadata.
pub(crate) fn slot(kind: DerivedKind) -> usize {
    kind.ordinal()
}

use ryo_derived::domain::contract::types::DerivedKind;
