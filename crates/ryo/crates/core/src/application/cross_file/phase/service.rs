//! The cross-file phase of a run: the project made now, made again for some rules or used again as the store kept it, the findings
//! the store kept of the rules not made now, and the rules that cross files made in their ready batches or in one ordinary phase.
use super::super::errors::CrossFileError;
use super::super::service::CrossFile;
use super::super::types::{
    ObservationRun, OrdinaryRun, PhaseOutcome, PhasePorts, PhaseRun, ProjectCrossing, Settling,
};
use crate::domain::answering::service::todo_list;
use crate::domain::answering::types::{Answered, Todo};
use crate::domain::budget::service::within;
use crate::domain::keeping::service::rendered_of;
use crate::domain::project::types::Projected;
use crate::domain::reporting::service::{later_from, unknowns_from};
use crate::domain::workers::types::{CrossReading, ProjectHandle};
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution as _;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::row_texts::types::PlacedRule;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stores::types::{MadeAgain, Reuse};
use std::sync::Arc;

impl CrossFile {
    /// The project and the rules that cross files: made now when every file was read (`made` is the project made now, when the
    /// rules read it), made again for the rules the store's decision names with the other rules' findings as the store kept them,
    /// or used again as the store kept them. Answers the project, and the ordinary phase's outcomes for the caller to settle.
    pub(crate) fn phase(
        &self,
        run: &PhaseRun<'_>,
        made: Option<Projected>,
        handed: CrossReading<'_, Box<dyn FactsWorker>>,
        handle: &ProjectHandle<Box<dyn FactsWorker>>,
        answered: &mut Answered,
        ports: PhasePorts<'_>,
    ) -> Result<PhaseOutcome, CrossFileError> {
        let as_kept = || Projected {
            refused: run.stored.refused.clone(),
            consulted: run.stored.consulted.clone(),
            consulted_absent: run.stored.consulted_absent.clone(),
            observed_entries: run.stored.project_observations.clone(),
            ..Projected::default()
        };
        let made_again: &[PlacedRule] = match run.again {
            Reuse::Cross(MadeAgain::These(rules)) => rules,
            Reuse::Again => {
                let later = later_from(run.stored, run.ids, &[]).unwrap_or_default();
                crate::domain::ordering::service::append(&mut answered.later, &rendered_of(&later));
                answered.unknowns.extend(unknowns_from(run.stored, run.read_again));
                answered.unknowns.extend(run.alone.iter().cloned());
                return Ok(PhaseOutcome {
                    project: as_kept(),
                    settling: Settling::Settled,
                });
            }
            Reuse::Whole | Reuse::Cross(MadeAgain::All | MadeAgain::Nothing) => {
                let mut now = made.unwrap_or_default();
                answered.unknowns.append(&mut now.unknowns);
                let todo = todo_list(&MetadataUseCaseImpl, run.metadata, run.ids);
                let settling = if todo.is_empty() {
                    Settling::Settled
                } else {
                    self.crossed(run, handed, &todo, handle, &now, answered, ports)?
                };
                return Ok(PhaseOutcome {
                    project: now,
                    settling,
                });
            }
        };
        // The cross-file steps of `made_again` alone, and the project unless a rule made again reads it.
        let ids: Vec<RuleId> = made_again
            .iter()
            .filter_map(|rule| run.ids.get(usize::try_from(rule.0).unwrap_or(usize::MAX)).cloned())
            .collect();
        let mut now = made.unwrap_or_else(as_kept);
        let later = later_from(run.stored, run.ids, &ids).unwrap_or_default();
        crate::domain::ordering::service::append(&mut answered.later, &rendered_of(&later));
        let mut todo = todo_list(&MetadataUseCaseImpl, run.metadata, run.ids);
        todo.retain(|each| {
            made_again
                .binary_search_by_key(&each.rule.0, |rule| usize::try_from(rule.0).unwrap_or(usize::MAX))
                .is_ok()
        });
        let settling = if todo.is_empty() {
            Settling::Settled
        } else {
            self.crossed(run, handed, &todo, handle, &now, answered, ports)?
        };
        answered.unknowns.append(&mut now.unknowns);
        Ok(PhaseOutcome {
            project: now,
            settling,
        })
    }

    /// Makes the rules that cross files. They are independent of one another: each is made by one worker, with names and tables of
    /// its own, from what the files handed over; the named observations run in their ready batches, the ordinary rules in one phase
    /// whose outcomes are answered.
    #[expect(
        clippy::too_many_arguments,
        reason = "what the run hands the cross-file steps, the project they read and the ports they finish and are guarded by arrive from the phase"
    )]
    fn crossed(
        &self,
        run: &PhaseRun<'_>,
        mut handed: CrossReading<'_, Box<dyn FactsWorker>>,
        todo: &[Todo],
        handle: &ProjectHandle<Box<dyn FactsWorker>>,
        now: &Projected,
        answered: &mut Answered,
        ports: PhasePorts<'_>,
    ) -> Result<Settling, CrossFileError> {
        let _crossing = tracing::info_span!("cross").entered();
        let unknown = ports.language.unknown_reasons();
        let kept_facts = ports
            .facts
            .finish(&mut std::mem::take(&mut handed.workers).into_iter())?;
        // The manifests the rows of the project are about that the run does not list are numbered after the files it lists.
        let across: Vec<FilePath> = run
            .paths
            .iter()
            .cloned()
            .chain(now.unlisted.iter().map(|each| FilePath(each.0.clone())))
            .collect();
        let domain = crate::domain::crossing::types::CrossingInput {
            placed: run.placed,
            observations: handed.observations,
            metadata: run.metadata,
            ids: run.ids,
            needs: run.needs,
            paths: &across,
            files: run.files,
            covered: handed.covered,
            skipped: handed.skipped,
            unread: handed.unread,
            project: &now.partials,
            answering: run.answering,
            unknown: &unknown.0,
        };
        let memory = ports.memory;
        if let Some(batches) = run.program.observation_batches() {
            for each in todo {
                within(memory.resident_bytes()?, run.budget, || vec![each.id.clone()])?;
            }
            self.observations(
                ObservationRun {
                    domain,
                    services: run.services,
                    program: Arc::clone(run.program),
                    batches,
                    first_bucket: run.first_bucket,
                    facts: kept_facts.as_ref(),
                    project: handle.facts.as_ref(),
                    project_symbols: &handle.symbols,
                    todo,
                },
                handed.arenas,
                answered,
                |rule| {
                    let id = run.ids.get(rule.0).ok_or(OperatingError::NotRun)?;
                    Ok(within(memory.resident_bytes()?, run.budget, || vec![id.clone()])?)
                },
            )?;
            return Ok(Settling::Settled);
        }
        let ordinary = self.ordinary(
            OrdinaryRun {
                crossing: ProjectCrossing {
                    program: Arc::clone(run.program),
                    arenas: &*handed.arenas,
                    domain,
                },
                services: run.services,
                todo,
                first_bucket: run.first_bucket,
                facts: kept_facts.as_ref(),
                project: handle.facts.as_ref(),
                project_symbols: &handle.symbols,
                ids: run.ids,
                budget: run.budget,
            },
            memory,
        )?;
        Ok(ordinary.map_or(Settling::Settled, Settling::Ordinary))
    }
}
