//! The ports the lint tests build over the engine's own use cases: publication, facts, operators and derived views.

use std::sync::Arc;

use ryo_execution::inbound::use_cases::prepare::interface::DerivedAccess;
use ryo_execution::inbound::use_cases::publication::interface::PublicationUseCase;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;

/// The engine's publication use case.
#[must_use]
pub fn publication() -> Arc<dyn PublicationUseCase> {
    Arc::new(
        ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new(),
    )
}

/// The digest port over the engine's digest fake.
#[must_use]
pub fn digest() -> Arc<dyn DigestPort> {
    Arc::new(DigestFake)
}

/// The facts port over the engine's in-memory fact store.
#[must_use]
pub fn facts() -> Arc<dyn FactsPort> {
    Arc::new(ryo_execution_runtime::outbound::facts::factory::memory())
}

/// The operators port over the engine's operators use case.
#[must_use]
pub fn operators() -> Arc<dyn OperatorsPort> {
    Arc::new(
        ryo_execution_runtime::outbound::operators::factory::use_case(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
                Arc::new(ryo_operators::outbound::matching::factory::use_case()),
            ),
        ),
    )
}

/// The derived access over the engine's derived use cases, as the prepare use case's composition builds it.
#[must_use]
pub fn derived() -> DerivedAccess {
    ryo_execution::derived()
}
