//! The rules of one file run in the engine's one call; the worker renders the emitted roots, notes the frontiers it could not
//! capture, keeps the captured ones it records and adds the counts.
use ryo_execution::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use ryo_execution::application::file_dag::program::prepared_execution::interface::PreparedExecution as _;
use super::super::errors::FileAnalysisError;
use super::super::semantic::service::SemanticState;
use super::super::service::FileAnalysis;
use super::super::types::{FileVisit, RuleGate, Worker};
use crate::domain::render::types::Answering;
use crate::domain::visit::types::Keeping;
use crate::domain::workers::service::governs_rule;
use crate::domain::workers::types::Tables;
use ryo_execution::application::file_dag::program::file_session::types::{
    CaptureContext, EmittedRoot, FileEnd, FileRunInput, FileStop, FinishedReadiness, KindSupport,
    RanRule, RuleAfter, SemanticSupport, SourceLanguage,
};
use ryo_execution::application::file_dag::program::preparation::types::Demand;
use ryo_execution::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::{
    ApplicabilityReason, ApplicabilityRefusal, Arena, CaptureIntent, DiagnosticDelivery,
    FileRuleIntent, ObservationDemand, OperatingError,
};
use ryo_execution::inbound::use_cases::metadata::interface::{
    FileObservationReadiness, Governing, Message, RuleIndex, Unknown,
};
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::elements::service::syntax_acceptance;
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::runs::types::{RuleApplicability, RuleNumber};
use std::sync::Arc;

impl FileAnalysis {
    /// The rest of a visit of a file the language read: the engine runs the per-file steps of the rules that govern it on its rows
    /// in one call, the words of what leaves it wait in the buckets, and the rows are released.
    pub(in crate::application::file_analysis) fn read_file(
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        file: FileId,
        semantic: &SemanticState,
    ) -> Result<(), FileAnalysisError> {
        let tables = &mut worker.tables;
        let acceptance = syntax_acceptance(&tables.elements);
        let original_mask = tables.mask.clone();
        let skipped = Self::gate_rules(tables, &visit.setting, file, acceptance);
        tables.mask = original_mask;
        // Publication always sees physical partiality, even when Rows output relaxes evaluation.
        let physical = tables.partials.clone();
        let evaluation = if matches!(visit.setting.answering, Answering::Rows(_)) {
            &[][..]
        } else {
            physical.as_slice()
        };
        let intents = intents_of(tables, visit, &skipped, acceptance);
        let ran = visit.setting.services.run_file(
            FileRunInput {
                file,
                evaluation,
                publication: &physical,
                intents: &intents,
                source: SourceView(&tables.bytes.0),
                elements: &tables.elements,
                language: if tables.language.is_some() {
                    SourceLanguage::Recognized
                } else {
                    SourceLanguage::Unrecognized
                },
                semantic: SemanticSupport::Supplied(semantic),
                kinds: match tables.kind_vocabulary.as_ref() {
                    Some(vocabulary) => KindSupport::Prepared(Arc::clone(vocabulary)),
                    None => KindSupport::Absent,
                },
                first: visit.setting.first_bucket,
            },
            &mut tables.symbols,
            CaptureContext {
                worker: tables.number,
                arena: Arena(&mut tables.arena.0),
                places: &mut tables.places,
                words: &mut tables.words,
            },
            worker.facts.as_ref(),
        );
        for RanRule { rule, emit, after } in ran.rules {
            if let EmittedRoot::Delivered(outcome) = emit {
                Self::delivered(tables, visit, file, semantic, rule, outcome)?;
            }
            let (captures, fed) = match after {
                RuleAfter::Ran { captures, fed } => (captures, fed),
                RuleAfter::Stopped(stop) => return Err(stopped(stop)),
                RuleAfter::Unreached => return Err(OperatingError::NotRun.into()),
            };
            for captured in captures {
                let capture = match captured.outcome {
                    Ok(capture) => capture,
                    Err(error) => {
                        tables.unknowns.push(Unknown {
                            rule: visit.setting.ids[rule.0].clone(),
                            path: visit.setting.paths[file.0 as usize].clone(),
                            message: Message(
                                format!("file phase frontier input {}: {error}", captured.input.0)
                                    .into(),
                            ),
                        });
                        continue;
                    }
                };
                if visit.setting.keeping == Keeping::Keeps
                    && *visit.setting.answering == Answering::Findings
                {
                    tables
                        .captured
                        .push(ryo_values::domain::records::types::Kept {
                            bucket: capture.bucket,
                            proof: capture.proof,
                            words: ryo_values::domain::stored_facts::types::KeptWords(
                                capture.words,
                            ),
                        });
                }
            }
            let counted = tables.fed.get_mut(rule.0).ok_or(OperatingError::NotRun)?;
            if counted.len() != fed.len() {
                return Err(OperatingError::ShapeMismatch.into());
            }
            for (total, current) in counted.iter_mut().zip(fed.iter()) {
                for (total, current) in total.0.iter_mut().zip(current.0) {
                    *total += current;
                }
            }
        }
        tables.current_observations = match ran.end {
            FileEnd::Finished(FinishedReadiness::Observed(readiness)) => Some(readiness),
            FileEnd::Finished(FinishedReadiness::Unobserved) => None,
            FileEnd::Unfinished => return Err(OperatingError::NotRun.into()),
            FileEnd::Refused(stop) => return Err(stopped(stop)),
        };
        if let Some(root) = &tables.current_observations {
            tables.observations.push(FileObservationReadiness {
                file,
                root: Arc::clone(root),
            });
        }
        crate::domain::reading::service::settle(tables, visit.setting.answering);
        worker.facts.release()?;
        Ok(())
    }

    /// The emitted root of `rule` in the file: its rows rendered, the files its partial rows could not decide noted, and what it
    /// could not decide or failed of noted or answered.
    fn delivered(
        tables: &mut Tables,
        visit: &FileVisit<'_>,
        file: FileId,
        semantic: &SemanticState,
        rule: RuleIndex,
        outcome: RootOutcome,
    ) -> Result<(), FileAnalysisError> {
        let affected = match &outcome {
            RootOutcome::Partial { affected, .. } => Some(affected.clone()),
            _ => None,
        };
        match outcome {
            RootOutcome::Complete(table) | RootOutcome::Partial { table, .. } => {
                let number = RuleNumber(u32::try_from(rule.0).map_err(|_| OperatingError::NotRun)?);
                let id = crate::domain::render::types::FindingRule(
                    visit
                        .setting
                        .ids
                        .get(rule.0)
                        .ok_or(OperatingError::NotRun)?
                        .0
                        .as_ref(),
                );
                let scene = crate::domain::value_cells::types::Scene {
                    file,
                    symbols: &tables.symbols,
                    elements: &tables.elements,
                    source: SourceView(&tables.bytes.0),
                    paths: visit.setting.paths,
                    unknown: visit.setting.unknown,
                };
                crate::domain::render::service::render_rows(
                    crate::domain::render::types::RenderInput {
                        rule: number,
                        table: &table,
                        emit: visit.setting.program.emit(rule)?,
                        layout: crate::domain::render::service::shown_by(
                            visit.setting.answering,
                            number,
                            id,
                        ),
                        scene,
                        limit: crate::domain::render::service::rendered_limit(
                            visit.setting.answering,
                        ),
                    },
                    &mut tables.starts,
                    &mut tables.rendered,
                )
                .map_err(|error| {
                    ryo_values::domain::rule_requests::service::failed(
                        &visit.setting.ids[rule.0],
                        &error,
                    )
                })?;
                if let Some(affected) = &affected {
                    crate::domain::recording::service::partial_root(
                        &MetadataUseCaseImpl,
                        &mut tables.unknowns,
                        &tables.partials,
                        &visit.setting,
                        rule,
                        file,
                        &visit.setting.paths[file.0 as usize],
                        affected,
                    );
                }
            }
            RootOutcome::Unknown(error) => {
                tables.unknowns.push(Unknown {
                    rule: visit.setting.ids[rule.0].clone(),
                    path: visit.setting.paths[file.0 as usize].clone(),
                    message: Message(error.to_string().into()),
                });
                if matches!(error, OperatingError::SemanticIncomplete { .. }) {
                    semantic.mark_incomplete();
                }
            }
            RootOutcome::Failed(error) => {
                return Err(ryo_values::domain::rule_requests::service::failed(
                    &visit.setting.ids[rule.0],
                    &error,
                )
                .into());
            }
            RootOutcome::Skipped(_) | RootOutcome::NoQuery | RootOutcome::Deferred { .. } => {}
        }
        Ok(())
    }
}

/// What the file asks of each rule: undemanded, skipped or refused by its applicability, or run with its diagnostic collected when
/// the rule governs the file.
fn intents_of(
    tables: &Tables,
    visit: &FileVisit<'_>,
    skipped: &[RuleGate],
    acceptance: SyntaxAcceptance,
) -> Vec<FileRuleIntent> {
    let demanded = visit.setting.program.demanded_rules(|rule| {
        if governs_rule(&MetadataUseCaseImpl, tables, rule) == Governing::Governs {
            Demand::Demanded
        } else {
            Demand::Undemanded
        }
    });
    demanded
        .iter()
        .enumerate()
        .map(|(place, demanded)| {
            if *demanded != Demand::Demanded {
                return FileRuleIntent::Undemanded;
            }
            if skipped[place] == RuleGate::Blocked {
                return match (visit.setting.applicability.get(place), acceptance) {
                    (Some(RuleApplicability::AcceptedSyntax), SyntaxAcceptance::Rejected) => {
                        FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected)
                    }
                    (Some(RuleApplicability::AcceptedSyntax), _) => {
                        FileRuleIntent::Refused(ApplicabilityRefusal::SyntaxUndetermined)
                    }
                    _ => FileRuleIntent::Refused(ApplicabilityRefusal::MetadataUnavailable),
                };
            }
            let emit =
                governs_rule(&MetadataUseCaseImpl, tables, RuleIndex(place)) == Governing::Governs;
            FileRuleIntent::Run {
                diagnostic: if emit {
                    DiagnosticDelivery::Collected
                } else {
                    DiagnosticDelivery::Disabled
                },
                observations: ObservationDemand::Required,
                capture: if emit {
                    CaptureIntent::DiagnosticAndObservations
                } else {
                    CaptureIntent::Observations
                },
            }
        })
        .collect()
}

/// The failure a stopped file's run answers: the engine's own, or the worker's facts'.
fn stopped(stop: FileStop) -> FileAnalysisError {
    match stop {
        FileStop::Operating(error) => error.into(),
        FileStop::Facts(error) => error.into(),
    }
}
