//! Static request interpretation and immutable access; no acquisition or retained policy.
use super::types::contract::value::{DerivedContract, kind_of};
use crate::domain::operating::errors::OperatingError;
use ryo_derived::domain::request::types::DerivedKind;

/// Checks that a contract is of the expected derived kind and that its metadata was admitted.
pub fn validate(
    expected: DerivedKind,
    contract: &DerivedContract,
) -> Result<(), OperatingError> {
    if contract.kind != expected || kind_of(&contract.request) != expected {
        return Err(OperatingError::WrongOperation);
    }
    contract.metadata.clone()
}
