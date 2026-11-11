//! Completion of a project phase: the diagnostic answer rendered and the named roots packed for publication.
use super::constants::INCOMPLETE_FACTS;
use super::constants::NO_RESERVED_ARENA;
use super::types::CrossedWithPublications;
use super::types::OutputPublication;
use super::types::ProgramEvaluated;
use crate::application::cross_file::publication::pack_roots;
use crate::application::cross_file::rendering::service::Rendering;
use crate::application::cross_file::types::ProjectCrossing;
use crate::domain::answering::types::Crossed;
use crate::domain::answering::types::Todo;
use crate::domain::crossing::types::ObservedRoot;
use crate::domain::crossing::types::PhaseRoots;
use ryo_execution::inbound::use_cases::execution::interface::ObservationContract;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramPreparedExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProjectObservation;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution::inbound::use_cases::publication::interface::PublicationUseCase;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::WorkerNumber;
use ryo_values::domain::records::types::ObservationReason;
use ryo_values::domain::records::types::ProducerReadiness;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use ryo_values::domain::rule_requests::service::failed;
use std::sync::Arc;

/// Delivers a project phase's answer and publications, over the engine's publication use case.
pub(crate) struct ProjectCompletion {
    /// Packs the demanded roots into the shared publication form.
    publication: Arc<dyn PublicationUseCase>,
    /// Places and renders the diagnostic root.
    rendering: Rendering,
}

impl ProjectCompletion {
    /// Bind the publication use case and the rendering once at composition.
    #[must_use]
    pub(crate) fn new(rendering: Rendering, publication: Arc<dyn PublicationUseCase>) -> Self {
        Self {
            publication,
            rendering,
        }
    }
    /// Renders the diagnostic root of the phase and packs its named roots for publication.
    pub(crate) fn complete_phase(
        &self,
        crossing: &ProjectCrossing<'_>,
        todo: &Todo,
        evaluated: ProgramEvaluated,
        cross: &mut Symbols,
        worker: Option<WorkerNumber>,
    ) -> Result<CrossedWithPublications, RuleRequestsError> {
        let emit = crossing
            .program
            .emit(todo.rule)
            .map_err(|error| failed(&todo.id, &error))?;
        let refuse = |reason: &dyn std::fmt::Display| failed(&todo.id, reason);
        let roots = evaluated.roots;
        if let Some(error) = &roots.emit_error
            && crossing.files.is_empty()
        {
            return Err(refuse(error));
        }
        let mut unknowns = if let Some(error) = &roots.emit_error {
            crossing
                .paths
                .iter()
                .take(crossing.files.len())
                .map(
                    |path| ryo_execution::inbound::use_cases::metadata::interface::Unknown {
                        rule: todo.id.clone(),
                        path: path.clone(),
                        message: ryo_execution::inbound::use_cases::metadata::interface::Message(
                            format!("project phase diagnostic emit: {error}").into(),
                        ),
                        cause: ryo_values::domain::runs::types::UnknownCause::Undecided,
                    },
                )
                .collect()
        } else {
            Vec::new()
        };
        if let Some(affected) = &roots.emit_affected {
            for index in MetadataUseCaseImpl
                .affected_ordinals(
                    ryo_execution::inbound::use_cases::metadata::types::AffectedFiles {
                        affected,
                        file_count:
                            ryo_execution::inbound::use_cases::metadata::types::CoverageFileCount(
                                crossing.files.len(),
                            ),
                    },
                )
                .map_err(|error| refuse(&error))?
                .0
            {
                let path = crossing.paths.get(index).ok_or_else(|| {
                    refuse(&ryo_execution::inbound::use_cases::execution::interface::OperatingError::NotRun)
                })?;
                // The file's own relation read in part already gives the rule's unknown there, with the relation and why.
                if crossing.covered.iter().any(|each| {
                    usize::try_from(each.file.0) == Ok(index)
                        && each.rules.contains(&todo.rule)
                        && crate::domain::recording::service::names_at_file(&MetadataUseCaseImpl,
                            &each.partial,
                            crossing.needs.get(todo.rule.0),
                        ) == ryo_execution::inbound::use_cases::metadata::interface::Bearing::Matters
                }) {
                    continue;
                }
                // A file the rule skipped, its syntax rejected, is skipped: it lacks the rule's input without the rule being
                // undecided there.
                if crossing
                    .skipped
                    .iter()
                    .any(|skip| skip.rule == todo.id && skip.path == *path)
                {
                    continue;
                }
                // A file the rule was refused on already gives the rule's unknown there, with why.
                if crossing
                    .refused
                    .get(&todo.id)
                    .is_some_and(|paths| paths.contains(path))
                {
                    continue;
                }
                unknowns.push(
                    ryo_execution::inbound::use_cases::metadata::interface::Unknown {
                        rule: todo.id.clone(),
                        path: path.clone(),
                        message: ryo_execution::inbound::use_cases::metadata::interface::Message(
                            INCOMPLETE_FACTS.into(),
                        ),
                        cause: ryo_values::domain::runs::types::UnknownCause::Undecided,
                    },
                );
            }
        }
        let rendered = match self
            .rendering
            .render_emit(crossing, todo, emit, roots.emit, cross)
        {
            Ok(rendered) => rendered,
            Err(error)
                if crossing
                    .program
                    .has_observation_contract(todo.rule)
                    .map_err(|reason| failed(&todo.id, &reason))?
                    == ObservationContract::Declared =>
            {
                unknowns.extend(
                    crossing
                        .paths
                        .iter()
                        .take(crossing.files.len())
                        .map(|path| {
                            ryo_execution::inbound::use_cases::metadata::interface::Unknown {
                                rule: todo.id.clone(),
                                path: path.clone(),
                                message:
                                    ryo_execution::inbound::use_cases::metadata::interface::Message(
                                        error.to_string().into(),
                                    ),
                                cause: ryo_values::domain::runs::types::UnknownCause::Undecided,
                            }
                        }),
                );
                None
            }
            Err(error) => return Err(error),
        };
        let observations = if roots.observations.is_empty() {
            None
        } else {
            let Some(worker) = worker else {
                return Err(refuse(&NO_RESERVED_ARENA));
            };
            Some(pack_roots(
                roots.observations,
                cross,
                crossing.arenas,
                worker,
                self.publication.as_ref(),
            ))
        };
        let mut publications = evaluated.publications;
        if let Some(packed) = &observations {
            publications.retain(|each| {
                packed.roots.iter().any(|root| {
                    root.output == each.output
                        && matches!(
                            root.words,
                            ryo_values::domain::records::types::RootWords::Published(_)
                        )
                })
            });
        } else {
            publications.clear();
        }
        Ok(CrossedWithPublications {
            crossed: Crossed {
                unknowns,
                rendered,
                observations,
            },
            publications,
        })
    }
}

/// The rows each root of a project rule's outcome delivers, and the owner proof of every usable named root.
pub(crate) fn collected(
    outcome: ryo_execution::inbound::use_cases::execution::interface::RuleOutcome<
        ryo_execution::inbound::use_cases::execution::interface::ProjectOutputOutcome,
    >,
) -> Result<ProgramEvaluated, OperatingError> {
    let mut roots = PhaseRoots::default();
    match outcome.emit {
        RootOutcome::Complete(table) => roots.emit = Some(table),
        RootOutcome::Partial { table, affected } => {
            roots.emit = Some(table);
            roots.emit_affected = Some(affected);
        }
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => roots.emit_error = Some(error),
        RootOutcome::Skipped(_) | RootOutcome::NoQuery | RootOutcome::Deferred { .. } => {}
    }
    let mut publications = Vec::new();
    for ryo_execution::inbound::use_cases::execution::interface::ProjectOutputOutcome {
        output,
        outcome,
    } in outcome.observations
    {
        let (table, readiness) = match outcome {
            ProjectObservation::Complete { table, publication } => {
                publications.push(OutputPublication {
                    output,
                    publication,
                });
                (Some(table), ProducerReadiness::Complete)
            }
            ProjectObservation::Partial { table, publication } => {
                publications.push(OutputPublication {
                    output,
                    publication,
                });
                (
                    Some(table),
                    ProducerReadiness::Unknown(ObservationReason(INCOMPLETE_FACTS.into())),
                )
            }
            ProjectObservation::Other(RootOutcome::Partial { .. }) => (
                None,
                ProducerReadiness::Unknown(ObservationReason(INCOMPLETE_FACTS.into())),
            ),
            ProjectObservation::Other(RootOutcome::Unknown(error) | RootOutcome::Failed(error)) => {
                (
                    None,
                    ProducerReadiness::Unknown(ObservationReason(
                        format!("project phase named output {}: {error}", output.0).into(),
                    )),
                )
            }
            ProjectObservation::Other(RootOutcome::Skipped(_)) => {
                (None, ProducerReadiness::Skipped)
            }
            ProjectObservation::Other(RootOutcome::NoQuery) => (None, ProducerReadiness::NoQuery),
            ProjectObservation::Other(RootOutcome::Complete(_) | RootOutcome::Deferred { .. }) => {
                return Err(OperatingError::WrongOperation);
            }
        };
        roots.observations.push(ObservedRoot {
            output,
            table,
            readiness,
        });
    }
    Ok(ProgramEvaluated {
        roots,
        publications,
    })
}
