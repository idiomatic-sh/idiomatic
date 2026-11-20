//! Thin public orchestration over the one private execution owner.
use super::interface::PrepareUseCase;
use super::types::{HarnessLanguage, HarnessSetting, PlanSource, PrepareRequest};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::application::file_scan::service::both;
use ryo_execution_runtime::application::file_scan::types::{HarnessRows, ScanReport};
use ryo_execution_runtime::application::pack_load::service::harnessed;
use ryo_execution_runtime::application::pack_load::types::{Harness, HarnessRequest};
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use std::sync::Arc;

/// Prepares programs over the operators it is given.
#[derive(Debug)]
pub struct PrepareUseCaseImpl {
    operators: Arc<dyn OperatorsPort>,
    /// What the edge composed for the build branch's `RYO_ENGINE` harness, when it composed the new engine.
    harness: Option<HarnessSetting>,
}

impl PrepareUseCaseImpl {
    /// A use case over the engine's operators.
    pub fn new(operators: Arc<dyn OperatorsPort>) -> Self {
        Self {
            operators,
            harness: None,
        }
    }

    /// The same, with the new engine the build branch's `RYO_ENGINE` harness loads each request's rules into.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution::inbound::use_cases::prepare::types::HarnessSetting;
    /// use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
    ///
    /// fn with_the_new_engine(prepare: PrepareUseCaseImpl, composed: HarnessSetting) -> PrepareUseCaseImpl {
    ///     prepare.harnessing(composed)
    /// }
    /// ```
    #[must_use]
    pub fn harnessing(mut self, harness: HarnessSetting) -> Self {
        self.harness = Some(harness);
        self
    }
}

impl PrepareUseCase for PrepareUseCaseImpl {
    fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError> {
        let compiled = match request.plans {
            PlanSource::Authored { plans, bindings } => {
                ryo_execution_domain::domain::prepared_request::service::assemble(
                    plans,
                    bindings,
                    &request.rules,
                )?
            }
            PlanSource::Compiled(queries) => queries.inner,
        };
        // The `RYO_ENGINE` harness of the build branch, before any file: nothing for the old engine. The lint store has no table
        // for a rule set's pack yet: none is read back, and the pack of the load is let go.
        let plans: Vec<&ryo_values::domain::plans::types::Plan> =
            compiled.plans.iter().map(AsRef::as_ref).collect();
        let loads = harnessed(
            self.harness.as_ref().map(|composed| Harness {
                engine: composed.engine.as_ref(),
                digest: composed.digest.as_ref(),
                vocabularies: &composed.vocabularies,
                named: match &composed.language {
                    HarnessLanguage::OnlyLinked => None,
                    HarnessLanguage::Named(name) => Some(name),
                },
            }),
            HarnessRequest {
                plans: &plans,
                digests: &request.digests,
                stored: &[],
            },
        )?;
        for load in &loads {
            // `both` while no linked language walks: the new engine ran no file, so its report is empty, and so are the old
            // engine's rows of the rules it accepted, which are none.
            let _compared = both(
                &HarnessRows::default(),
                &ScanReport::default(),
                &load.accepted,
            )?;
        }
        ryo_execution_runtime::application::file_dag::service::prepare(
            compiled,
            &request.rules,
            &request.digests,
            &request.hosted,
            request.observations,
            Arc::clone(&self.operators),
        )
    }
}
