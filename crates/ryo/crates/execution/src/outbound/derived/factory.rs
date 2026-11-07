//! Composition edge for the portable implementation.
use crate::inbound::use_cases::execution::types::DerivedPort;
#[must_use]
pub fn use_case(
    semantics: impl ryo_derived::inbound::use_cases::derived::interface::DerivedUseCase
    + Send
    + Sync
    + std::fmt::Debug,
) -> impl DerivedPort {
    semantics
}
