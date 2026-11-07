//! The ports the lint tests build over the engine's own use cases: publication, operators and derived views.

use std::sync::Arc;

use crate::outbound::publication::interface::PublicationPort;
use ryo_execution::inbound::use_cases::execution::types::{DerivedPort, OperatorsPort};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;

/// The publication port over the engine's publication use case.
#[must_use]
pub fn publication() -> Arc<dyn PublicationPort> {
    Arc::new(crate::outbound::publication::factory::execution(Arc::new(
        ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new(),
    )))
}

/// The operators port over the engine's operators use case.
#[must_use]
pub fn operators() -> Arc<dyn OperatorsPort> {
    Arc::new(ryo_execution::outbound::operators::factory::use_case(
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
            Arc::new(ryo_operators::outbound::matching::factory::use_case()),
        ),
    ))
}

/// The derived-views port over the engine's derived use case.
#[must_use]
pub fn derived() -> Arc<dyn DerivedPort> {
    Arc::new(ryo_execution::outbound::derived::factory::use_case(
        ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
    ))
}

/// The metadata use case the engine decides over request metadata with.
#[must_use]
pub fn metadata() -> Arc<dyn MetadataUseCase + Send + Sync> {
    Arc::new(ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl)
}
