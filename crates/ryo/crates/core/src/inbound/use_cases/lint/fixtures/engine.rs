//! The ports the lint tests build over the engine's own use cases: publication, facts and operators; and the analysis of a run
//! of the new engine alone, over the engine port's fake.

use std::sync::Arc;

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
            ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl::new(),
            ryo_operators::inbound::use_cases::run::use_case::RunUseCaseImpl::new(
                std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()),
            ),
        ),
    )
}

/// The analysis of a run of the new engine alone: `analysis` with the engine port's fake composed, which runs a rule of the one
/// tree rule's shape and refuses every other, and the new engine said to run the files whatever the environment selects.
#[must_use]
pub fn new_engine(
    analysis: crate::application::file_analysis::service::FileAnalysis,
) -> crate::application::file_analysis::service::FileAnalysis {
    analysis
        .scanning(Arc::new(
            ryo_execution_runtime::outbound::engine::fixtures::fake::EngineFake::default(),
        ))
        .running(ryo_operators::spike::EngineMode::New)
}

/// The analysis of a run of the new engine alone in which a rule passes its work allowance: as `new_engine`, the fake
/// noticing of the rule it runs, in the second file of the run, a join with no key of a thousand rows with a thousand.
#[must_use]
pub fn noticing_engine(
    analysis: crate::application::file_analysis::service::FileAnalysis,
) -> crate::application::file_analysis::service::FileAnalysis {
    use ryo_execution_runtime::outbound::engine::fixtures::fake::{EngineFake, overrun};
    let second = ryo_relations::domain::relations::types::FileId(1);
    analysis
        .scanning(Arc::new(EngineFake::noticing(second, overrun())))
        .running(ryo_operators::spike::EngineMode::New)
}
