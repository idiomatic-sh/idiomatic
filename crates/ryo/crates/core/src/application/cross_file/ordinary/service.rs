//! The ordinary project phase: the shared stage opened once, each declared rule's scope and frontiers prepared within the budget,
//! the sealed batch evaluated and completed, and the stage released.
use super::super::constants::MISSING_FRONTIER_BUCKET;
use super::super::errors::CrossFileError;
use super::super::preparation::service::Preparation;
use super::super::preparation::types::PreparedProject;
use super::super::service::CrossFile;
use super::super::types::{OrdinaryOutcome, OrdinaryRun};
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use crate::domain::budget::service::within;
use crate::outbound::memory::interface::MemoryPort;
use ryo_execution::application::file_dag::frontiers::errors::PreparationError;
use ryo_execution::application::file_dag::frontiers::types::PublicationSlot;
use ryo_execution::application::file_dag::program::project_phase::interface::ProjectStage;
use ryo_execution::application::file_dag::program::project_phase::types::{
    FrontierDemand, FrontierRequest,
};
use ryo_execution::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use ryo_execution::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rule_requests::service::failed;

impl CrossFile {
    /// Makes the ordinary rules that cross files in one phase: they are independent of one another and share one interpreter and
    /// symbol generation. Answers each rule's outcome in the order of `run.todo`, the refusal of the budget guard, and the release
    /// of the stage, for the caller to settle.
    pub(crate) fn ordinary(
        &self,
        run: OrdinaryRun<'_>,
        memory: &dyn MemoryPort,
    ) -> Result<Option<OrdinaryOutcome>, CrossFileError> {
        let Some(first_rule) = run.todo.first() else {
            return Ok(None);
        };
        within(memory.resident_bytes()?, run.budget, || vec![first_rule.id.clone()])?;
        let stage = run
            .services
            .project_phase()
            .map_err(|error| failed(&first_rule.id, &error))?;
        // All ordinary project rules use the same interpreter and symbol generation.
        let mut cross = Symbols::default();
        let crossing = &run.crossing;
        let inapplicable = crate::domain::crossing::service::inapplicable_files(
            &MetadataUseCaseImpl,
            &crossing.domain,
        );
        let prepared = Preparation::ordinary(run.todo).prepare_all(|occurrence, each| {
            if occurrence.0 != 0 {
                within(memory.resident_bytes()?, run.budget, || vec![each.id.clone()])?;
            }
            let first = *run.first_bucket.get(each.rule.0).ok_or_else(|| {
                CrossFileError::from(failed(&each.id, &MISSING_FRONTIER_BUCKET))
            })?;
            let scope = stage
                .project_scope(
                    each.rule,
                    crossing.domain.files,
                    crossing.domain.covered,
                    crossing.domain.skipped,
                    inapplicable
                        .get(each.rule.0)
                        .ok_or(OperatingError::NotRun)
                        .map_err(|error| failed(&each.id, &error))?,
                )
                .map_err(|error| failed(&each.id, &error))?;
            let frontier = run
                .services
                .prepare_frontiers(
                    FrontierRequest {
                        scope: &scope,
                        rule: each.rule,
                        first,
                        facts: run.facts,
                        project: run.project,
                        project_symbols: run.project_symbols,
                        demand: FrontierDemand::Diagnostic,
                    },
                    &mut cross,
                )
                .map_err(|error| match error {
                    PreparationError::Facts(error) => CrossFileError::from(error),
                    PreparationError::Metadata(error) => CrossFileError::from(failed(&each.id, &error)),
                })?;
            Ok(PreparedProject {
                scope,
                frontier,
                bound: Vec::new(),
                slot: PublicationSlot::Unused,
            })
        });
        let batch = prepared.seal()?;
        let guarded = self.evaluate_batch(crossing, batch, &stage, &mut cross, |rule| {
            let id = run.ids.get(rule.0).ok_or(OperatingError::NotRun)?;
            Ok(within(memory.resident_bytes()?, run.budget, || vec![id.clone()])?)
        });
        let (outcomes, refused) = match guarded {
            ProjectBatch::Complete(completed) => (completed, None),
            ProjectBatch::Refused { completed, error } => (completed, Some(error)),
        };
        let symbols = cross;
        // Always release the shared stage; the caller keeps an earlier rule or budget failure over a cleanup refusal.
        let finished = stage
            .finish()
            .map_err(|error| CrossFileError::from(failed(&first_rule.id, &error)));
        Ok(Some(OrdinaryOutcome {
            outcomes,
            refused,
            finished,
            symbols,
        }))
    }
}
