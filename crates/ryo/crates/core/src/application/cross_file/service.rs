//! Cross-file evaluation of the project batches over injected ports.
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use super::completion::service::ProjectCompletion;
use super::completion::service::collected;
use super::completion::types::CrossedWithPublications;
use super::constants::MISSING_OUTCOME;
use super::errors::CrossFileError;
use super::preparation::service::DeclaredProjectPreparation;
use super::preparation::service::Preparation;
use super::preparation::service::SealedBatch;
use super::preparation::types::PreparedProject;
use super::rendering::service::Rendering;
use super::types::ObservationRun;
use super::types::ProjectCrossing;
use super::types::ProjectOutcome;
use super::types::ReadyProject;
use crate::domain::answering::service::settle;
use crate::domain::answering::types::Answered;
use crate::domain::answering::types::Todo;
use crate::domain::crossing::service::inapplicable_files;
use crate::domain::crossing::service::partials_for;
use crate::domain::crossing::service::partials_for_reads;
use crate::domain::render::types::Answering;
use crate::outbound::corpus::interface::CorpusPort;
use ryo_execution::outbound::publication::interface::PublicationPort;
use ryo_execution::inbound::use_cases::execution::interface::InputReadiness;
use ryo_execution::inbound::use_cases::execution::interface::{AdmittedOutput, RuleSelection};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
use ryo_execution::inbound::use_cases::execution::interface::ObservationPublication;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::EmitDemand;
use ryo_execution::inbound::use_cases::execution::interface::FileObservationReadiness;
use ryo_execution::inbound::use_cases::execution::interface::FrontierDemand;
use ryo_execution::inbound::use_cases::execution::interface::FrontierRequest;
use ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectInput;
use ryo_execution::inbound::use_cases::execution::interface::ProjectTask;
use ryo_execution::inbound::use_cases::execution::interface::RuleDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rule_requests::service::failed;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use std::sync::Arc;

/// Cross-file work sharing the caller's corpus resource.
pub struct CrossFile {
    /// Renders each phase's answer and packs its publications.
    completion: ProjectCompletion,
}

impl std::fmt::Debug for CrossFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl CrossFile {
    /// Bind source access once at composition.
    #[must_use]
    pub fn new(corpus: Arc<dyn CorpusPort>, publication: Arc<dyn PublicationPort>) -> Self {
        Self {
            completion: ProjectCompletion::new(Rendering::new(corpus), publication),
        }
    }
    /// The sealed application inventory is the required input to engine work.
    pub(crate) fn evaluate_batch(
        &self,
        crossing: &ProjectCrossing<'_>,
        batch: SealedBatch,
        stage: &impl ProjectRootExecution,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), CrossFileError>,
    ) -> ryo_execution::inbound::use_cases::execution::interface::ProjectBatch<
        ProjectOutcome,
        CrossFileError,
    > {
        let mut entries = batch
            .into_entries()
            .into_vec()
            .into_iter()
            .map(|DeclaredProjectPreparation { todo, result: task }| {
                let prepared = task.and_then(
                    |PreparedProject {
                         scope,
                         frontier,
                         bound,
                         slot,
                     }| {
                        let evaluation = match crossing.answering {
                            Answering::Findings => partials_for(&MetadataUseCaseImpl, &crossing.domain, todo.rule),
                            Answering::Rows(_) => Default::default(),
                        };
                        let reads = crossing
                            .program
                            .observation_reads(todo.rule)
                            .map_err(|error| CrossFileError::from(failed(&todo.id, &error)))?;
                        let publication = partials_for_reads(&MetadataUseCaseImpl, &crossing.domain, todo.rule, &reads);
                        Ok(ReadyProject {
                            scope,
                            frontier: Some(frontier),
                            bound,
                            slot,
                            evaluation,
                            publication,
                        })
                    },
                );
                (todo, prepared)
            })
            .collect::<Vec<_>>();
        let inputs = entries
            .iter_mut()
            .filter_map(|(todo, prepared)| {
                let Ok(ready) = prepared else {
                    return None;
                };
                Some(ProjectTask {
                    rule: todo.rule,
                    demand: RuleDemand {
                        emit: EmitDemand::Enabled,
                        observations: ObservationDemand::Required,
                    },
                    input: ProjectInput {
                        scope: &ready.scope,
                        evaluation: &ready.evaluation,
                        publication: &ready.publication,
                        ordinary: ready
                            .frontier
                            .take()
                            .expect(std::any::type_name::<ReadyProject>()),
                        bound: &ready.bound,
                        arenas: crossing.arenas,
                    },
                })
            })
            .collect();
        let (completed, refused) = match stage.evaluate(inputs, symbols, guard) {
            ProjectBatch::Complete(completed) => (completed, None),
            ProjectBatch::Refused { completed, error } => (completed, Some(error)),
        };
        let mut evaluated = completed.into_iter();
        let mut completed = Vec::with_capacity(entries.len());
        for (todo, prepared) in entries {
            let result = match prepared {
                Err(error) => Err(error),
                Ok(ready) => {
                    let Some(outcome) = evaluated.next() else {
                        if refused.is_some() {
                            break;
                        }
                        completed.push(ProjectOutcome {
                            result: Err(CrossFileError::from(failed(&todo.id, &MISSING_OUTCOME))),
                            todo,
                        });
                        continue;
                    };
                    (|| {
                        let outcome = outcome
                            .map_err(|error| CrossFileError::from(failed(&todo.id, &error)))?;
                        let evaluated = collected(outcome)
                            .map_err(|error| CrossFileError::from(failed(&todo.id, &error)))?;
                        self.completion
                            .complete_phase(crossing, &todo, evaluated, symbols, ready.slot)
                            .map_err(CrossFileError::from)
                    })()
                }
            };
            completed.push(ProjectOutcome { todo, result });
        }
        match refused {
            Some(error) => ProjectBatch::Refused { completed, error },
            None => ProjectBatch::Complete(completed),
        }
    }

    /// Execute the declared ready batches and transfer publications before their final consumer retires.
    pub(crate) fn observations(
        &self,
        run: ObservationRun<'_>,
        arenas: &mut Vec<ArenaBytes>,
        answered: &mut Answered,
        mut guard: impl FnMut(RuleIndex) -> Result<(), CrossFileError>,
    ) -> Result<(), CrossFileError> {
        let mut stage = run.services.observations(arenas)?;
        let mut selected = vec![RuleSelection::Unselected; run.domain.metadata.len()];
        for todo in run.todo {
            *selected
                .get_mut(todo.rule.0)
                .ok_or(OperatingError::NotRun)? = RuleSelection::Selected;
        }
        for (rule, selected) in selected.iter().enumerate() {
            if *selected == RuleSelection::Unselected {
                stage.finish(RuleIndex(rule))?;
            }
        }
        let readiness = run
            .domain
            .observations
            .iter()
            .map(|frame| FileObservationReadiness {
                file: frame.file,
                root: Arc::clone(&frame.root),
            })
            .collect::<Vec<_>>();
        let inapplicable = inapplicable_files(&MetadataUseCaseImpl, &run.domain);
        stage.restore_frontiers(
            run.facts,
            run.domain.files,
            run.domain.covered,
            run.domain.skipped,
            &inapplicable,
            &readiness,
            run.first_bucket,
            &selected,
        )?;
        // The retained project execution and every decoded text place share one
        // live symbol generation across all producer and consumer batches.
        let mut symbols = Symbols::default();
        for batch in run.batches {
            let inventory = Preparation::named(batch, run.todo);
            if inventory.declaration_count().0 == 0 {
                continue;
            }
            let prepared = inventory.prepare_all(|_, todo| {
                prepare_project(&run, &mut stage, &inapplicable, &mut symbols, todo)
            });
            let sealed = prepared.seal()?;
            let retiring = sealed.declarations();
            let (results, refused) = {
                let crossing = ProjectCrossing {
                    arenas: stage.arenas(),
                    program: Arc::clone(&run.program),
                    domain: run.domain.clone(),
                };
                match self.evaluate_batch(&crossing, sealed, &stage, &mut symbols, &mut guard) {
                    ProjectBatch::Complete(completed) => (completed, None),
                    ProjectBatch::Refused { completed, error } => (completed, Some(error)),
                }
            };
            let mut first_error = None;
            let mut outcomes = results.into_iter();
            for todo in retiring.iter() {
                if let Some(outcome) = outcomes.next() {
                    let delivered = outcome
                        .result
                        .and_then(|result| deliver(&mut stage, todo, result, answered));
                    if let Err(error) = delivered {
                        first_error.get_or_insert(error);
                    }
                }
                if let Err(error) = stage.finish(todo.rule) {
                    first_error.get_or_insert(CrossFileError::from(error));
                }
            }
            if let Some(error) = refused {
                first_error.get_or_insert(error);
            }
            if let Some(error) = first_error {
                let _finished = stage.finish_stage();
                return Err(error);
            }
        }
        stage.finish_stage()?;
        Ok(())
    }
}

/// Settle one delivered occurrence into the answers and install its publications on the stage.
fn deliver(
    stage: &mut impl ObservationPublication,
    todo: &Todo,
    result: CrossedWithPublications,
    answered: &mut Answered,
) -> Result<(), CrossFileError> {
    let Some(transfer) = settle(
        todo,
        result.crossed,
        &mut answered.later,
        &mut answered.fed,
        &mut answered.unknowns,
    ) else {
        return Ok(());
    };
    let publications = result
        .publications
        .into_iter()
        .map(|each| AdmittedOutput {
            output: each.output,
            publication: each.publication,
        })
        .collect();
    stage.install(todo.rule, result.slot, transfer, publications)?;
    Ok(())
}

/// Reserve, bind and scope one declared occurrence, then prepare its observation frontiers.
fn prepare_project(
    run: &ObservationRun<'_>,
    stage: &mut impl ObservationExecution,
    inapplicable: &[InapplicableFiles],
    symbols: &mut Symbols,
    todo: &Todo,
) -> Result<PreparedProject, CrossFileError> {
    let readiness = stage.binding_readiness(todo.rule)?;
    let dependencies = run.program.frontier_dependencies(todo.rule)?;
    let frontier_ready = dependencies
        .iter()
        .map(|inputs| {
            if inputs
                .iter()
                .all(|input| readiness.get(input).copied() == Some(InputReadiness::Ready))
            {
                InputReadiness::Ready
            } else {
                InputReadiness::Unavailable
            }
        })
        .collect::<Vec<_>>();
    let slot = stage.reserve(todo.rule)?;
    let bound = stage.bindings(todo.rule)?;
    let scope = stage.project_scope(
        todo.rule,
        run.domain.files,
        run.domain.covered,
        run.domain.skipped,
        inapplicable
            .get(todo.rule.0)
            .ok_or(OperatingError::NotRun)?,
    )?;
    let refuse = |error: &dyn std::fmt::Display| failed(&todo.id, error);
    let first = *run
        .first_bucket
        .get(todo.rule.0)
        .ok_or_else(|| refuse(&OperatingError::NotRun))?;
    let frontier = run
        .services
        .prepare_frontiers(
            FrontierRequest {
                scope: &scope,
                rule: todo.rule,
                first,
                facts: run.facts,
                project: run.project,
                project_symbols: run.project_symbols,
                demand: FrontierDemand::Observations {
                    ready: &frontier_ready,
                },
            },
            symbols,
        )
        .map_err(|error| refuse(&error))?;
    Ok(PreparedProject {
        scope,
        frontier,
        bound,
        slot,
    })
}
