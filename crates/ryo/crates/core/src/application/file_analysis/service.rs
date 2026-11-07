//! Bounded read, parse and per-file evaluation using injected ports.
use crate::application::file_analysis::constants::{
    EXECUTION_DIAGNOSTIC, INPUT_EDGE_DID_NOT_SUPPLY_EXECUTABLE,
    PROVIDER_PARSE,
    RULE_SKIPPED_APPLICABILITY_METADATA_UNAVAILABLE, RULE_SKIPPED_SYNTAX_ACCEPTANCE_UNDETERMINED,
    STORED_FRONTIER_PROOF_DOES_NOT_MATCH,
};
use ryo_execution::inbound::use_cases::execution::interface::{
    ExecutionRequest, FileExecution, PreparedExecution,
};
use ryo_execution::inbound::use_cases::execution::types::{
    ApplicabilityReason, ApplicabilityRefusal, CaptureContext, CaptureIntent, DiagnosticDelivery,
    EmitDemand, FileInput, FileRuleIntent, ObservationDemand, RootOutcome, RuleDemand,
    SourceLanguage, SourceRows,
};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
// this operation owns one bounded worker and the read, parse, admission, evaluation, and retention sequence for its file; its private methods share that invocation state, while domain computations remain separate pure functions.
use super::errors::FileAnalysisError;
use super::types::{FileCompleteness, FileVisit, RequestVisitSetting, RuleGate, Worker};
use crate::domain::admission::errors::AdmissionError;
use crate::domain::admission::service::{admit_elements, admit_view, limits_of};

use crate::domain::budget::service::within;
use ryo_execution::inbound::use_cases::metadata::interface::{Recovery, RuleIndex};

use crate::domain::handing::service::rows_of_file;
use crate::domain::keeping::service::{remap_packed, rows_of};
use crate::domain::keeping::types::Remap;
use crate::domain::project::types::Projected;
use crate::domain::reading::service::{
    begin, engine_asked, failure_partials, prepare, read_failure_demanded, settle as settle_file,
    start_partials,
};
use crate::domain::reading::types::{
    EngineAsked, EngineRows, FailureDetail, ObservedFile, PrimaryFailureKind, ReadFailureDemand,
    RunRuleCount,
};
use crate::domain::recording::service::{
    cover, finish_file, frontier_flat, interface_bytes, made_ran, moved_words, replay,
};
use crate::domain::handing::service::{hand_rest_again, remap_to};
use crate::domain::recording::types::Flat;
use crate::domain::render::types::Answering;
use crate::domain::workers::types::{Gating, Held, Ready, ReadyBucket, Tables};
use ryo_relations::domain::relations::types::Words;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::records::types::{
    GoverningRules, HandedNotes, Handing, Record, RuleFrontier,
};
use ryo_values::domain::row_texts::types::PlacedRule;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket};
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::BucketsWanted;
use ryo_values::domain::table_words::service::pack_onto;
use ryo_values::domain::table_words::types::WordSlice;

use crate::domain::rules::service::governing;

use crate::domain::visit::service::lines_in;
use crate::domain::visit::types::{BorrowedBytes, Keeping};

use crate::domain::workers::service::{checking, file_at, govern, governs_rule};
use ryo_values::domain::runs::types::RuleNumber;

use crate::domain::workers::types::{Checking, FileOrder};
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::PathStat;
use ryo_execution::inbound::use_cases::execution::types::digest::DigestPort;
use ryo_execution::inbound::use_cases::execution::types::digest::{DigestBytes, DigestWords};
use ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker;
use ryo_execution::inbound::use_cases::execution::types::facts::WordsToKeep;
use ryo_relations::domain::relations::collections::relations::filesystem_executable::item::FilesystemExecutableRow;
use ryo_values::domain::files::service::input_path;
use ryo_values::domain::files::types::ExecutablePermission;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::stored_facts::types::{PartialAt, PartialCause, PartialScope, ReadInPart};

use crate::outbound::language::interface::LanguagePort;
use crate::outbound::language::types::{
    FileReading, FileToRead, ProjectToRead, SourceSelection,
};

use crate::outbound::memory::interface::MemoryPort;

use crate::outbound::scheduler::types::JobNumber;
use ryo_language::domain::elements::service::{
    has_incomplete_lexical_stream, has_incomplete_syntax, syntax_acceptance,
};
use ryo_language::domain::diagnostics::types::SyntaxCompleteness;
use ryo_language::domain::elements::types::{Elements, SyntaxAcceptance};
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::symbols::service::clear_symbols;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::collections::relations::filesystem_file::item::FilesystemFileRow;
use ryo_relations::domain::relations::collections::relations::filesystem_line_count::item::FilesystemLineCountRow;
use ryo_relations::domain::relations::types::Rows as _;
use ryo_relations::domain::relations::types::{FileId, Integer, Relation};

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, CorpusInput, FileBytes, FileNumber};
use ryo_values::domain::stores::types::StoredFile;

use ryo_values::domain::records::service::is_as_it_was;
use ryo_values::domain::records::types::{FileOutcome, Freshness, Handed, Ran, Unreadable};

use ryo_values::domain::runs::types::ManifestRefused;
use ryo_values::domain::runs::types::RuleApplicability;

use ryo_values::domain::stored_facts::types::{ArenaBase, Digest, LanguageNumber, ReadFailure};
use std::borrow::Cow;
use std::sync::Arc;
use std::time::Instant;

/// Whether the language read a complete tree or a recovery from missing syntax.
pub(super) fn recovery_of(reading: &FileReading, elements: &Elements) -> Recovery {
    match reading {
        FileReading::Read(_) if has_incomplete_syntax(elements) == SyntaxCompleteness::Incomplete => {
            if has_incomplete_lexical_stream(elements) == SyntaxCompleteness::Incomplete {
                Recovery::SyntaxErrors
            } else {
                Recovery::LexicallyCompleteSyntaxErrors
            }
        }
        FileReading::Failed(_) => Recovery::Unparsed,
        FileReading::Read(_) | FileReading::Manifest | FileReading::Unsupported => Recovery::Whole,
    }
}

/// Analysis of one file or the project, with resources supplied by the composition.
pub struct FileAnalysis {
    /// Reads each file's bytes.
    corpus: Arc<dyn CorpusPort>,
    /// Parses a file into its syntax tree and rows, and names the languages linked.
    language: Arc<dyn LanguagePort>,
    /// Digests the files whose results the store keeps between runs.
    digest: Arc<dyn DigestPort>,
    /// Says how much memory the process holds, which a file's visit keeps within the run's budget.
    memory: Arc<dyn MemoryPort>,
}

impl std::fmt::Debug for FileAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl Worker {
    /// The application creates the worker's bounded request-metadata cache.
    pub(crate) fn new(tables: Tables, facts: Box<dyn FactsWorker>) -> Self {
        Self {
            tables,
            facts,
            old_rows: None,
            failure: None,
        }
    }
}

impl FileAnalysis {
    /// Admit one exact linked-provider selection before parsing or retained-data reuse.
    pub(crate) fn validate_source_language(
        &self,
        selected: Option<&ryo_values::domain::descriptions::types::LanguageName>,
    ) -> Result<(), FileAnalysisError> {
        if let Some(selected) = selected
            && self
                .language
                .kind_vocabularies()
                .0
                .iter()
                .filter(|vocabulary| vocabulary.language == *selected)
                .count()
                != 1
        {
            return Err(FileAnalysisError::SourceLanguage {
                name: selected.0.to_string(),
            });
        }
        Ok(())
    }

    /// The parsed relations needed by rules that govern this path and whose applicability is met.
    pub(super) fn active_wanted<'a>(
        tables: &Tables,
        setting: &'a RequestVisitSetting<'_>,
        acceptance: SyntaxAcceptance,
    ) -> Cow<'a, [Relation]> {
        if setting.metadata.is_empty() {
            return Cow::Borrowed(setting.wanted);
        }
        // All rules are active for legacy plans, and complete metadata plus accepted syntax
        // activates every policy. Keep the caller's relation slice in both cases.
        if setting.applicability_can_gate == Gating::NeverGates
            || acceptance == SyntaxAcceptance::Accepted
                && setting.applicability.len() == setting.metadata.len()
        {
            return Cow::Borrowed(setting.wanted);
        }
        let demanded = setting
            .program
            .demanded_rules(|place| governs_rule(tables, RuleIndex(place)) == Governing::Governs);
        Cow::Owned(
            setting
                .wanted
                .iter()
                .copied()
                .filter(|relation| {
                    setting.metadata.iter().enumerate().any(|(place, _)| {
                        demanded.get(place).copied().unwrap_or(false)
                            && match setting.applicability.get(place).copied() {
                                Some(RuleApplicability::Always) => true,
                                Some(RuleApplicability::AcceptedSyntax) => {
                                    acceptance == SyntaxAcceptance::Accepted
                                }
                                None => false,
                            }
                            && setting.needs.get(place).is_some_and(|needs| {
                                needs.reads.iter().any(|read| read.relation == *relation)
                            })
                    })
                })
                .collect(),
        )
    }

    /// Whether this setting contains a policy that can change a file's active rules.
    pub(super) fn may_gate_rules(setting: &RequestVisitSetting<'_>, acceptance: SyntaxAcceptance) -> Gating {
        if setting.applicability_can_gate == Gating::MayGate
            && !(acceptance == SyntaxAcceptance::Accepted
                && setting.applicability.len() == setting.metadata.len())
        {
            Gating::MayGate
        } else {
            Gating::NeverGates
        }
    }

    /// Bind the resources used by every analysis invocation.
    #[must_use]
    pub fn new(
        corpus: Arc<dyn CorpusPort>,
        language: Arc<dyn LanguagePort>,
        digest: Arc<dyn DigestPort>,
        memory: Arc<dyn MemoryPort>,
    ) -> Self {
        Self {
            corpus,
            language,
            digest,
            memory,
        }
    }

    /// Admits what the language `provider` wrote for the file `number`, against the file's length, its table and its number.
    fn admit(
        &self,
        worker: &Worker,
        wanted: &[Relation],
        number: FileId,
        provider: LanguageNumber,
    ) -> Result<(), FileAnalysisError> {
        let refused = |code: AdmissionError| FileAnalysisError::Provider {
            provider: self
                .language
                .linked()
                .0
                .get(provider.0)
                .map(|name| name.0.to_string())
                .unwrap_or_default(),
            code,
        };
        let tables = &worker.tables;
        let limits = limits_of(&tables.elements, &tables.bytes, number);
        admit_elements(&tables.elements, &limits).map_err(refused)?;
        for relation in wanted {
            admit_view(&worker.facts.rows(*relation)?, &limits).map_err(refused)?;
        }
        Ok(())
    }

    /// Removes rules whose declared syntax applicability is not met and records why each one was
    /// skipped for this file. The skip is separate from relation partiality: it says the rule was
    /// not applicable, not that its input was read incompletely.
    pub(super) fn gate_rules(
        tables: &mut Tables,
        setting: &RequestVisitSetting<'_>,
        file: FileId,
        acceptance: SyntaxAcceptance,
    ) -> Vec<RuleGate> {
        let demanded = setting
            .program
            .demanded_rules(|place| governs_rule(tables, RuleIndex(place)) == Governing::Governs);
        let mut skipped = vec![RuleGate::Open; setting.metadata.len()];
        for (place, blocked) in skipped.iter_mut().enumerate() {
            if !demanded.get(place).copied().unwrap_or(false) {
                continue;
            }
            let applicability = setting.applicability.get(place).copied();
            *blocked = match applicability {
                Some(RuleApplicability::Always) => RuleGate::Open,
                Some(RuleApplicability::AcceptedSyntax) => {
                    if acceptance == SyntaxAcceptance::Accepted {
                        RuleGate::Open
                    } else {
                        RuleGate::Blocked
                    }
                }
                None => RuleGate::Blocked,
            };
            if *blocked == RuleGate::Open {
                continue;
            }
            MetadataUseCaseImpl.unplace(
                &mut tables.mask,
                RuleNumber(u32::try_from(place).unwrap_or(u32::MAX)),
            );
            let reason = match (applicability, acceptance) {
                (Some(RuleApplicability::AcceptedSyntax), SyntaxAcceptance::Rejected) => {
                    let (Some(rule), Some(path)) = (
                        setting.ids.get(place),
                        setting
                            .paths
                            .get(usize::try_from(file.0).unwrap_or(usize::MAX)),
                    ) else {
                        continue;
                    };
                    tables
                        .skipped
                        .push(ryo_values::domain::runs::types::Skipped {
                            rule: rule.clone(),
                            path: path.clone(),
                            reason: ryo_values::domain::runs::types::SkipReason::SyntaxRejected,
                        });
                    continue;
                }
                (Some(RuleApplicability::AcceptedSyntax), SyntaxAcceptance::Undetermined) => {
                    RULE_SKIPPED_SYNTAX_ACCEPTANCE_UNDETERMINED
                }
                (Some(RuleApplicability::AcceptedSyntax), SyntaxAcceptance::Accepted)
                | (Some(RuleApplicability::Always), _) => continue,
                (None, _) => RULE_SKIPPED_APPLICABILITY_METADATA_UNAVAILABLE,
            };
            let Some(rule) = setting.ids.get(place) else {
                continue;
            };
            let Some(path) = setting
                .paths
                .get(usize::try_from(file.0).unwrap_or(usize::MAX))
            else {
                continue;
            };
            tables.unknowns.push(
                ryo_execution::inbound::use_cases::metadata::interface::Unknown {
                    rule: rule.clone(),
                    path: path.clone(),
                    message: ryo_execution::inbound::use_cases::metadata::interface::Message(
                        reason.into(),
                    ),
                },
            );
        }
        skipped
    }

    /// The rest of a visit of a file the language read: the per-file steps of the rules that govern it run on its rows, the words of
    /// what leaves it wait in the buckets, and the rows are released.
    fn read_file(
        &self,
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        file: FileId,
        semantic: &super::semantic::SemanticState,
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
        let demanded = visit
            .setting
            .program
            .demanded_rules(|place| governs_rule(tables, RuleIndex(place)) == Governing::Governs);
        let intents = demanded
            .iter()
            .enumerate()
            .map(|(place, demanded)| {
                if !demanded {
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
                let emit = governs_rule(tables, RuleIndex(place)) == Governing::Governs;
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
            .collect::<Vec<_>>();
        let refused = std::cell::RefCell::new(None);
        let read = |relation| {
            worker
                .facts
                .rows(relation)
                .map(SourceRows::Stored)
                .map_err(|error| {
                    *refused.borrow_mut() = Some(error);
                    ryo_execution::inbound::use_cases::execution::types::OperatingError::NotRead
                })
        };
        let mut run = visit.setting.services.open_file(
            FileInput {
                file,
                evaluation,
                publication: &physical,
                intents: &intents,
                source: &tables.bytes.0,
                elements: &tables.elements,
                language: if tables.language.is_some() {
                    SourceLanguage::Recognized
                } else {
                    SourceLanguage::Unrecognized
                },
                resident_stability: worker.facts.resident_stability(),
                read: &read,
                semantic: Some(semantic),
                kinds: tables.kind_vocabulary.as_ref().map(Arc::clone),
            },
            &mut tables.symbols,
        )?;
        for rule in visit.setting.program.rule_order() {
            let Some(FileRuleIntent::Run {
                diagnostic,
                observations,
                ..
            }) = intents.get(rule.0)
            else {
                continue;
            };
            let emit = *diagnostic != DiagnosticDelivery::Disabled;
            let demand = RuleDemand {
                emit: if emit {
                    EmitDemand::Enabled
                } else {
                    EmitDemand::Disabled
                },
                observations: *observations,
            };
            let outcome = { run.rule(rule, demand, &mut tables.symbols) };
            if let Some(error) = refused.take() {
                return Err(error.into());
            }
            let outcome = outcome?;
            // Named roots have already been published independently of diagnostic delivery.
            if emit {
                let affected = match &outcome.emit {
                    RootOutcome::Partial { affected, .. } => Some(affected.clone()),
                    _ => None,
                };
                match outcome.emit {
                    RootOutcome::Complete(table) | RootOutcome::Partial { table, .. } => {
                        let number = RuleNumber(u32::try_from(rule.0).map_err(|_| {
                            ryo_execution::inbound::use_cases::execution::types::OperatingError::NotRun
                        })?);
                        let id = crate::domain::render::types::FindingRule(
                            visit
                                .setting
                                .ids
                                .get(rule.0)
                                .ok_or(ryo_execution::inbound::use_cases::execution::types::OperatingError::NotRun)?
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
                        tables.unknowns.push(
                            ryo_execution::inbound::use_cases::metadata::interface::Unknown {
                                rule: visit.setting.ids[rule.0].clone(),
                                path: visit.setting.paths[file.0 as usize].clone(),
                                message:
                                    ryo_execution::inbound::use_cases::metadata::interface::Message(
                                        error.to_string().into(),
                                    ),
                            },
                        );
                        if matches!(
                            error,
                            ryo_execution::inbound::use_cases::execution::types::OperatingError::SemanticIncomplete { .. }
                        ) {
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
                    RootOutcome::Skipped(_)
                    | RootOutcome::NoQuery
                    | RootOutcome::Deferred { .. } => {}
                }
            }
            let captures = {
                run.capture_frontiers(
                    rule,
                    *visit.setting.first_bucket.get(rule.0).ok_or(
                        ryo_execution::inbound::use_cases::execution::types::OperatingError::NotRun,
                    )?,
                    &mut tables.symbols,
                    CaptureContext {
                        worker: tables.number,
                        arena: &mut tables.arena.0,
                        places: &mut tables.places,
                        words: &mut tables.words,
                    },
                )
            };
            if let Some(error) = refused.take() {
                return Err(error.into());
            }
            for captured in captures? {
                let capture = match captured.outcome {
                    Ok(capture) => capture,
                    Err(error) => {
                        tables.unknowns.push(
                            ryo_execution::inbound::use_cases::metadata::interface::Unknown {
                                rule: visit.setting.ids[rule.0].clone(),
                                path: visit.setting.paths[file.0 as usize].clone(),
                                message:
                                    ryo_execution::inbound::use_cases::metadata::interface::Message(
                                        format!(
                                            "file phase frontier input {}: {error}",
                                            captured.input.0
                                        )
                                        .into(),
                                    ),
                            },
                        );
                        continue;
                    }
                };
                worker.facts.keep(
                    capture.bucket,
                    WordsToKeep {
                        words: &capture.words,
                        file,
                        proof: &capture.proof,
                    },
                )?;
                run.commit_frontier(rule, capture.input)?;
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
            let counts = run.counts(rule)?;
            let counted = tables.fed.get_mut(rule.0).ok_or(
                ryo_execution::inbound::use_cases::execution::types::OperatingError::NotRun,
            )?;
            if counted.len() != counts.fed.len() {
                return Err(ryo_execution::inbound::use_cases::execution::types::OperatingError::ShapeMismatch.into());
            }
            for (total, current) in counted.iter_mut().zip(counts.fed.iter()) {
                for (total, current) in total.0.iter_mut().zip(current.0) {
                    *total += current;
                }
            }
        }
        tables.current_observations = run.finish()?.readiness;
        if let Some(root) = &tables.current_observations {
            tables.observations.push(
                ryo_execution::inbound::use_cases::metadata::interface::FileObservationReadiness {
                    file,
                    root: Arc::clone(root),
                },
            );
        }
        settle_file(tables, visit.setting.answering);
        worker.facts.release()?;
        Ok(())
    }

    /// Reads primary inventoried manifests once before project preparation, retaining genuine read failures.
    /// Consulted project dependencies use their separate lookup route and never become primary failure events.
    pub(crate) fn read_manifests(
        &self,
        files: &mut [CorpusInput],
    ) -> Result<
        Vec<ryo_language::inbound::use_cases::project::types::ManifestFile>,
        FileAnalysisError,
    > {
        use crate::outbound::language::types::ManifestReading;
        use ryo_language::inbound::use_cases::project::types::{ManifestFile, ManifestText};
        use ryo_values::domain::files::types::InputReadFailure;
        let mut manifests = Vec::new();
        let mut text = FileBytes::default();
        for input in files {
            if self.language.reads_manifest(input_path(input)) != ManifestReading::Read {
                continue;
            }
            let CorpusInput::Available(file) = input else {
                continue;
            };
            text.0.clear();
            match self.corpus.read(file, &mut text) {
                Ok(()) => manifests.push(ManifestFile {
                    path: file.path.clone(),
                    text: ManifestText(String::from_utf8_lossy(&text.0).into()),
                }),
                Err(CorpusError::Unreadable { reason, .. }) => {
                    text.0.clear();
                    *input = CorpusInput::ReadFailed {
                        entry: file.clone(),
                        reason: InputReadFailure(reason.into()),
                    };
                }
                Err(error) => {
                    text.0.clear();
                    return Err(error.into());
                }
            }
        }
        Ok(manifests)
    }

    /// The visit of one file, in the worker that took its job: read it, decide which rules govern it, parse it, and run their
    /// per-file steps on its rows. What it could not read is noted in the worker; what stops the run is its failure.
    pub(crate) fn visit_file(
        &self,
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        files: &[CorpusInput],
        job: JobNumber,
    ) {
        if let Err(error) = self.visit(worker, visit, files, job) {
            // The first failure is the one the run answers with.
            if worker.failure.is_none() {
                worker.failure = Some(error);
            }
        }
    }

    /// The visit of one file, answering what stops the run.
    fn visit(
        &self,
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        files: &[CorpusInput],
        job: JobNumber,
    ) -> Result<(), FileAnalysisError> {
        let setting = &visit.setting;
        let at = FileOrder(usize::try_from(job.0).unwrap_or(usize::MAX));
        let Some(file) = file_at(files, at) else {
            return Ok(());
        };
        let number = FileId(job.0);
        if govern(setting, &mut worker.tables, input_path(file)) == Governing::DoesNotGovern {
            return Ok(());
        }
        if checking(at) == Checking::Now {
            let mask = &worker.tables.mask;
            within(self.memory.resident_bytes()?, setting.budget, || {
                governing(mask, setting.ids)
            })?;
        }
        let file = match file {
            CorpusInput::Available(entry) => entry,
            CorpusInput::ReadFailed { entry, reason } => {
                return self.read_failed(
                    worker,
                    visit,
                    number,
                    &entry.path,
                    entry.executable,
                    FailureDetail(&reason.0),
                    PrimaryFailureKind::Disk,
                    ObservedFile::Observed,
                );
            }
            CorpusInput::Unreadable { path, reason } => {
                return self.read_failed(
                    worker,
                    visit,
                    number,
                    path,
                    ExecutablePermission::Unknown,
                    FailureDetail(&reason.0),
                    PrimaryFailureKind::Disk,
                    ObservedFile::Unobserved,
                );
            }
        };
        let kept = setting.records.get(at.0).copied().flatten();
        // A file the system says is as it was is not read: what came of it is used again.
        if let Some(record) = kept.filter(|record| {
            read_failure_demanded(
                setting.wanted,
                &worker.tables.mask,
                setting.needs,
                RunRuleCount(setting.ids.len()),
            ) == ReadFailureDemand::NotDemanded
                && is_as_it_was(record, file) == Freshness::AsItWas
        }) {
            replay(&mut worker.tables, number, at, record, file, record.digest);
            return Self::hand_again(worker, visit, at, number, record);
        }
        worker.tables.bytes.0.clear();
        if let Err(error) = self.corpus.read(file, &mut worker.tables.bytes) {
            worker.tables.bytes.0.clear();
            let kind = if matches!(error, CorpusError::Unreadable { .. }) {
                PrimaryFailureKind::Disk
            } else {
                PrimaryFailureKind::Other
            };
            return self.read_failed(
                worker,
                visit,
                number,
                &file.path,
                file.executable,
                FailureDetail(&error.to_string()),
                kind,
                ObservedFile::Observed,
            );
        }
        // Only a store reads the digest of a file's bytes.
        let digest =
            if *setting.answering == Answering::Findings && setting.keeping == Keeping::Keeps {
                self.digest.digest(DigestBytes(&worker.tables.bytes.0))
            } else {
                Digest([0; 16])
            };
        // A file written again with the same bytes is as it was too.
        if let Some(record) =
            kept.filter(|record| record.digest == digest && record.executable == file.executable)
        {
            replay(&mut worker.tables, number, at, record, file, digest);
            return Self::hand_again(worker, visit, at, number, record);
        }
        let outcome = self.parse_file(worker, visit, file, number)?;
        let outcome = Self::stage_outcome(visit.store, outcome)?;
        let acceptance = syntax_acceptance(&worker.tables.elements);
        cover(&mut worker.tables, setting, number, &file.path, acceptance);
        finish_file(
            &mut worker.tables,
            setting,
            at,
            file,
            digest,
            FileNumber(number.0),
            outcome,
        );
        Ok(())
    }

    /// Executes demanded engine facts for a primary input failure without constructing source or syntax.
    fn read_failed(
        &self,
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        number: FileId,
        path: &FilePath,
        executable: ExecutablePermission,
        detail: FailureDetail<'_>,
        kind: PrimaryFailureKind,
        observed_file: ObservedFile,
    ) -> Result<(), FileAnalysisError> {
        use ryo_language::domain::symbols::service::intern_derived;
        use ryo_language::domain::symbols::types::ByteSlice;
        use ryo_relations::domain::relations::collections::relations::filesystem_read_failure::item::FilesystemReadFailureRow;
        use ryo_relations::domain::relations::types::{Offset, Span};
        use ryo_relations::domain::terms::types::NameText;

        let setting = &visit.setting;
        worker.tables.bytes.0.clear();
        prepare(&mut worker.tables);
        clear_symbols(&mut worker.tables.symbols);
        worker.tables.language = None;
        worker.facts.release()?;
        let asked = engine_asked(setting.wanted);
        let mut said = failure_partials(
            setting.wanted,
            number,
            detail,
            observed_file,
        );
        Self::write_engine_rows(
            worker,
            EngineAsked {
                file: if observed_file == ObservedFile::Observed {
                    asked.file
                } else {
                    EngineRows::NotAsked
                },
                lines: EngineRows::NotAsked,
                ..asked
            },
            number,
            executable,
            &mut said,
        )?;
        if asked.read_failure == EngineRows::Asked && kind == PrimaryFailureKind::Disk {
            let detail =
                intern_derived(&mut worker.tables.symbols, ByteSlice(&[]), NameText(detail.0));
            worker.facts.columns_mut()?.write(FilesystemReadFailureRow {
                path: number,
                detail,
                span: Span {
                    start: Offset(0),
                    end: Offset(0),
                },
            });
        }
        start_partials(&mut worker.tables, &said, Recovery::Whole, setting.wanted);
        worker.tables.unread.push(Unread {
            path: path.clone(),
            reason: Unreadable::Failed(ReadFailure(detail.0.into())),
        });
        let acceptance = syntax_acceptance(&worker.tables.elements);
        if asked.any == EngineRows::Asked {
            index_kinds(&mut worker.tables.elements);
            let semantic =
                super::semantic::SemanticState::new(number, &worker.tables.elements, None);
            self.read_file(worker, visit, number, &semantic)?;
        } else if Self::may_gate_rules(setting, acceptance) == Gating::MayGate {
            drop(Self::gate_rules(
                &mut worker.tables,
                setting,
                number,
                acceptance,
            ));
        }
        cover(&mut worker.tables, setting, number, path, acceptance);
        // Failure facts are invocation-local: retry the real input next time even if metadata is unchanged.
        if matches!(setting.answering, Answering::Findings) {
            worker.tables.unrecorded.push(path.clone());
        }
        Ok(())
    }

    /// Only successfully validated and digested fresh outcomes cross this IO boundary.
    pub(super) fn stage_outcome(
        read: Option<&dyn crate::outbound::lint_store::interface::ReadStore>,
        mut outcome: Option<FileOutcome>,
    ) -> Result<Option<FileOutcome>, FileAnalysisError> {
        let Some(read) = read else {
            return Ok(outcome);
        };
        let ran = match outcome.as_mut() {
            Some(FileOutcome::Read(ran) | FileOutcome::Unread { ran: Some(ran), .. }) => Some(ran),
            None | Some(FileOutcome::Unread { ran: None, .. }) => None,
        };
        if let Some(ran) = ran
            && let Handing::Made(handed) = &mut ran.handed
        {
            for bucket in &mut handed.buckets {
                let words = std::mem::replace(
                    &mut bucket.words,
                    ryo_values::domain::records::types::RecordWords::Inline(PackedWords::default()),
                );
                bucket.words = match words {
                    ryo_values::domain::records::types::RecordWords::Inline(words) => {
                        read.stage_words(words)?
                    }
                    staged @ ryo_values::domain::records::types::RecordWords::Staged(_) => staged,
                };
            }
        }
        Ok(outcome)
    }

    /// Hands what a file the run uses again hands the steps that read across files, when they are made again: one path whether the
    /// record holds it (the file was read alone before the run) or the store kept it (read as it is handed, its words moved as they
    /// are unpacked, none copied first). The run is ready to be handed only when it makes those steps again.
    pub(super) fn hand_again(
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        at: FileOrder,
        number: FileId,
        record: &Record,
    ) -> Result<(), FileAnalysisError> {
        let setting = &visit.setting;
        if !matches!(
            setting.reuse,
            ryo_values::domain::stores::types::Reuse::Cross(_)
        ) {
            return Ok(());
        }
        let ran = match &record.outcome {
            FileOutcome::Read(ran) | FileOutcome::Unread { ran: Some(ran), .. } => ran,
            FileOutcome::Unread { ran: None, .. } => return Ok(()),
        };
        let map = remap_to(&worker.tables, number);
        let mut out = Words::default();
        let (no_text, no_governing) = (ArenaBytes::default(), GoverningRules::default());
        let ready = match &ran.handed {
            Handing::Made(handed) => Ready {
                text: &handed.text,
                notes: match &handed.notes {
                    HandedNotes::Noted(notes) => Some(notes),
                    HandedNotes::Nothing => None,
                },
                governing: &handed.governing,
                partials: &handed.partials,
                observations: match &handed.observations {
                    ryo_values::domain::records::types::HandedReadiness::Recorded(root) => {
                        Some(root)
                    }
                    ryo_values::domain::records::types::HandedReadiness::Unrecorded => None,
                },
                buckets: handed
                    .buckets
                    .iter()
                    .map(|each| {
                        let held = match &each.words {
                            ryo_values::domain::records::types::RecordWords::Inline(words) => {
                                Held::Words(words)
                            }
                            ryo_values::domain::records::types::RecordWords::Staged(span) => {
                                Held::Staged(span)
                            }
                        };
                        ReadyBucket { bucket: each.bucket, proof: &each.proof, held }
                    })
                    .collect(),
            },
            Handing::Kept => {
                let Ok(place) = setting
                    .stored
                    .files
                    .binary_search_by(|each| each.path.cmp(&record.path))
                else {
                    return Ok(());
                };
                let file = StoredFileNumber(u32::try_from(place).unwrap_or(u32::MAX));
                let kept = setting.handed;
                let covered = rows_of_file(&kept.covered, file, |each| each.file).first();
                Ready {
                    text: rows_of_file(&kept.texts, file, |each| each.file)
                        .first()
                        .map_or(&no_text, |each| &each.text),
                    notes: rows_of_file(&kept.notes, file, |each| each.file)
                        .first()
                        .map(|each| &each.notes),
                    governing: covered.map_or(&no_governing, |each| &each.governing),
                    partials: covered.map_or(&[][..], |each| &each.partials),
                    observations: match rows_of_file(&kept.observations, file, |each| each.file) {
                        [one] => Some(&one.root),
                        _ => None,
                    },
                    buckets: rows_of_file(&kept.frontier, file, |each| each.file)
                        .iter()
                        .map(|each| {
                            let held = match &each.words {
                                ryo_values::domain::stores::types::ReadWordPayload::Inline(
                                    words,
                                ) => Held::Words(words),
                                ryo_values::domain::stores::types::ReadWordPayload::Old(span) => {
                                    Held::Old(span)
                                }
                            };
                            ReadyBucket { bucket: each.bucket, proof: &each.proof, held }
                        })
                        .collect(),
                }
            }
        };
        hand_rest_again(
            &mut worker.tables,
            at,
            number,
            ready.text,
            ready.notes,
            ready.governing,
            ready.partials,
            ready.observations,
        );
        let mut loaded = PackedWords::default();
        for ReadyBucket { bucket, proof, held } in ready.buckets {
            // Filtering must precede any staged or old read, including an invalid unrequested descriptor.
            if let BucketsWanted::These(buckets) = setting.wanted_buckets
                && buckets.binary_search(&bucket).is_err()
            {
                continue;
            }
            let words = match held {
                Held::Words(words) => words,
                Held::Staged(span) => {
                    let read = visit
                        .store
                        .ok_or(crate::outbound::lint_store::errors::LintStoreError::NothingRead)?;
                    read.read_staged(span, &mut loaded)?;
                    &loaded
                }
                Held::Old(span) => {
                    let read = visit
                        .store
                        .ok_or(crate::outbound::lint_store::errors::LintStoreError::NothingRead)?;
                    if worker.old_rows.is_none() {
                        worker.old_rows = read.begin_old_rows(setting.wanted_buckets)?;
                    }
                    if let Some(batch) = worker.old_rows.as_mut() {
                        batch.read(span, &mut loaded)?;
                    } else {
                        read.read_old(span, &mut loaded)?;
                    }
                    &loaded
                }
            };
            Self::hand_words_again(
                worker, setting, number, bucket, proof, words, &map, &mut out,
            )?;
        }
        Ok(())
    }

    /// Hands the words a file handed the last run in `bucket`, moved as `map` says, to the worker's bucket, when the run makes the
    /// steps that read the bucket again; words that cannot be moved are left out. `out` is room the words are moved into.
    ///
    /// Order: both callers select the bucket first (`wanted_buckets`) and read no words, staged or old, of an unselected bucket, so
    /// this is reached only for a selected one. It checks the proof of that bucket against the prepared root (version, rule digest,
    /// frontier ordinal, capture flag, first bucket, coverage) before it moves any words, and refuses a stale one; the proof of an
    /// unselected bucket is never checked, as its words are never read.
    fn hand_words_again(
        worker: &mut Worker,
        setting: &RequestVisitSetting<'_>,
        file: FileId,
        bucket: Bucket,
        proof: &ryo_values::domain::records::types::FrontierProof,
        words: &PackedWords,
        map: &Remap,
        out: &mut Words,
    ) -> Result<(), FileAnalysisError> {
        use ryo_values::domain::records::types::FrontierCoverage;
        let rule = RuleIndex(usize::try_from(proof.rule.0).unwrap_or(usize::MAX));
        let input = usize::try_from(proof.input.0).unwrap_or(usize::MAX);
        let valid = setting
            .program
            .rule_digest(rule)
            .is_ok_and(|digest| digest == proof.rule_digest)
            && setting
                .program
                .frontiers(rule)
                .is_ok_and(|frontiers| frontiers.get(input).is_some())
            && setting
                .metadata
                .get(rule.0)
                .and_then(|rule| rule.frontier_capture.get(input))
                .copied()
                == Some(ryo_values::domain::run_metadata::types::FrontierFeed::Captured)
            && setting.first_bucket.get(rule.0).copied().map(|first| {
                ryo_values::domain::stored_facts::service::frontier_bucket(
                    first,
                    InputNumber(input),
                )
            }) == Some(bucket)
            && match &proof.coverage {
                FrontierCoverage::Complete => true,
                FrontierCoverage::Partial(affected) => {
                    ryo_values::domain::records::service::is_well_formed(affected)
                }
            };
        if !valid {
            return Err(
                ryo_execution::inbound::use_cases::execution::types::OperatingError::ObservationIncomplete {
                    reason: STORED_FRONTIER_PROOF_DOES_NOT_MATCH.into(),
                }
                .into(),
            );
        }
        out.0.clear();
        if remap_packed(words, map, out).is_some() {
            let packed = &mut worker.tables.packed;
            packed.0.clear();
            pack_onto(WordSlice(&out.0), packed);
            worker.facts.keep(
                bucket,
                WordsToKeep {
                    words: &worker.tables.packed,
                    file,
                    proof,
                },
            )?;
        }
        Ok(())
    }

    /// The read of a file the run has not kept: the language parses it, what it wrote is admitted, and the per-file steps run on
    /// its rows. Answers what the store keeps of it.
    fn parse_file(
        &self,
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        file: &CorpusEntry,
        number: FileId,
    ) -> Result<Option<FileOutcome>, FileAnalysisError> {
        let setting = &visit.setting;
        let base = begin(&mut worker.tables);
        let source = FileToRead {
            source_language: setting
                .source_language
                .map_or(SourceSelection::Recognized, SourceSelection::Named),
            path: &file.path,
            file: number,
            bytes: &worker.tables.bytes,
            wanted: setting.wanted,
            notes: setting.noted.into(),
            roles: setting.roled.into(),
        };
        let mut writer = worker.facts.columns_mut()?;
        let parse_started = tracing::enabled!(target: EXECUTION_DIAGNOSTIC, tracing::Level::TRACE)
            .then(Instant::now);
        let reading = self.language.parse(
            source,
            &mut worker.tables.elements,
            &mut worker.tables.symbols,
            &mut writer,
        );
        if let Some(started) = parse_started {
            tracing::trace!(target: EXECUTION_DIAGNOSTIC, phase = PROVIDER_PARSE, file = number.0, elapsed_ns = started.elapsed().as_nanos() as u64);
        }
        let mut said: Vec<ReadInPart> = std::mem::take(&mut writer.partials);
        let acceptance = syntax_acceptance(&worker.tables.elements);
        let active_wanted = Self::active_wanted(&worker.tables, setting, acceptance);
        // The language that read the file is kept with what it noted of it.
        worker.tables.language = match &reading {
            FileReading::Read(language) => Some(*language),
            FileReading::Manifest | FileReading::Unsupported | FileReading::Failed(_) => None,
        };
        // Rows a file left in the worker's tables before it was found unreadable are released here.
        if !matches!(reading, FileReading::Read(_)) {
            worker.facts.release()?;
        }
        // What the language wrote is admitted before the store keeps anything or a rule reads it.
        if let FileReading::Read(provider) = &reading {
            self.admit(worker, active_wanted.as_ref(), number, *provider)?;
        }
        let asked = engine_asked(active_wanted.as_ref());
        Self::write_engine_rows(worker, asked, number, file.executable, &mut said)?;
        // The relations this file is read in part: those a reader said so of, and all of code if what was read is a recovery.
        let everything = recovery_of(&reading, &worker.tables.elements);
        start_partials(
            &mut worker.tables,
            &said,
            everything,
            active_wanted.as_ref(),
        );
        let semantic_needed = setting.program.semantic_needed();
        let session = if semantic_needed {
            worker.tables.language.and_then(|language| {
                let source = FileToRead {
                    source_language: setting
                        .source_language
                        .map_or(SourceSelection::Recognized, SourceSelection::Named),
                    path: &file.path,
                    file: number,
                    bytes: &worker.tables.bytes,
                    wanted: setting.wanted,
                    notes: setting.noted.into(),
                    roles: setting.roled.into(),
                };
                self.language
                    .begin_semantics(language, source, &worker.tables.elements)
            })
        } else {
            None
        };
        let semantic =
            super::semantic::SemanticState::new(number, &worker.tables.elements, session);
        Ok(match reading {
            FileReading::Read(_) | FileReading::Manifest => {
                index_kinds(&mut worker.tables.elements);
                self.read_file(worker, visit, number, &semantic)?;
                if semantic.completeness() == FileCompleteness::Incomplete {
                    None
                } else {
                    self.recorded(worker, visit, number, base)
                        .map(FileOutcome::Read)
                }
            }
            unread @ (FileReading::Failed(_) | FileReading::Unsupported) => {
                // What no language read has no tree and no names; the rules that ask for the file still run.
                let reason = match unread {
                    FileReading::Failed(reason) => Unreadable::Failed(reason),
                    FileReading::Read(_) | FileReading::Manifest | FileReading::Unsupported => {
                        Unreadable::NoLanguage
                    }
                };
                worker.tables.unread.push(Unread {
                    path: file.path.clone(),
                    reason: reason.clone(),
                });
                if asked.any == EngineRows::Asked {
                    clear_symbols(&mut worker.tables.symbols);
                    index_kinds(&mut worker.tables.elements);
                    self.read_file(worker, visit, number, &semantic)?;
                } else {
                    let unavailable_acceptance = syntax_acceptance(&worker.tables.elements);
                    if Self::may_gate_rules(setting, unavailable_acceptance) == Gating::MayGate {
                        drop(Self::gate_rules(
                            &mut worker.tables,
                            setting,
                            number,
                            unavailable_acceptance,
                        ));
                    }
                }
                match (setting.answering, asked.any) {
                    (Answering::Rows(_), _) => None,
                    (Answering::Findings, EngineRows::NotAsked) => {
                        Some(FileOutcome::Unread { reason, ran: None })
                    }
                    (Answering::Findings, EngineRows::Asked) => {
                        if semantic.completeness() == FileCompleteness::Incomplete {
                            None
                        } else {
                            self.recorded(worker, visit, number, base).map(|ran| {
                                FileOutcome::Unread {
                                    reason,
                                    ran: Some(ran),
                                }
                            })
                        }
                    }
                }
            }
        })
    }

    /// Writes the rows the engine writes of every file, for the rules that ask for them.
    fn write_engine_rows(
        worker: &mut Worker,
        asked: EngineAsked,
        number: FileId,
        executable: ExecutablePermission,
        said: &mut Vec<ReadInPart>,
    ) -> Result<(), FileAnalysisError> {
        if asked.executable == EngineRows::Asked {
            match executable {
                posix @ (ExecutablePermission::PosixExecutable
                | ExecutablePermission::PosixNotExecutable) => {
                    worker.facts.columns_mut()?.write(FilesystemExecutableRow {
                        path: number,
                        executable: if posix == ExecutablePermission::PosixExecutable {
                            ryo_relations::domain::relations::types::Flag::Raised
                        } else {
                            ryo_relations::domain::relations::types::Flag::Lowered
                        },
                    })
                }
                ExecutablePermission::Unsupported => {}
                ExecutablePermission::Unknown => said.push(ReadInPart {
                    relation: Relation::FilesystemExecutable,
                    cause: PartialCause(INPUT_EDGE_DID_NOT_SUPPLY_EXECUTABLE.into()),
                    lacking: ryo_relations::domain::relations::types::Missing::Any,
                    at: PartialScope::OneFile(PartialAt(number.0)),
                }),
            }
        }
        if asked.file == EngineRows::Asked {
            worker
                .facts
                .columns_mut()?
                .write(FilesystemFileRow { path: number });
        }
        if asked.lines == EngineRows::Asked {
            let lines = Integer(lines_in(BorrowedBytes(&worker.tables.bytes.0)).0);
            worker.facts.columns_mut()?.write(FilesystemLineCountRow {
                path: number,
                lines,
            });
        }
        Ok(())
    }

    /// What a file read came to, as the store keeps it: its findings, and what it shows other files: what its language noted of it,
    /// the relations it is read in part and what it hands the cross-file steps. Nothing when the run keeps nothing, or the words cannot
    /// be moved to another run.
    pub(super) fn recorded(
        &self,
        worker: &Worker,
        visit: &FileVisit<'_>,
        number: FileId,
        base: ArenaBase,
    ) -> Option<Ran> {
        let (setting, tables) = (&visit.setting, &worker.tables);
        if matches!(setting.answering, Answering::Rows(_)) {
            return None;
        }
        // A run whose store keeps nothing makes its findings alone: no words moved for another run, no digest of what it shows.
        if setting.keeping == Keeping::Discards {
            return Some(Ran {
                rows: rows_of(&tables.rendered),
                number: FileNumber(number.0),
                interface: Digest([0; 16]),
                frontier: Vec::new(),
                noted: Digest([0; 16]),
                handed: Handing::Made(Handed::default()),
            });
        }
        let moved = moved_words(tables, number, base)?;
        let layout = setting.program.observation_layout();
        let (observations, slots) = match (tables.current_observations.as_deref(), layout) {
            (Some(map), Some((layout, slots))) => {
                let Ok(slot_count) = u32::try_from(slots.len()) else {
                    return None;
                };
                let count =
                    ryo_values::domain::records::types::ObservationReadinessCount(slot_count);
                let Ok(checked) = ryo_values::domain::observations::service::validate_frontiers(
                    map, layout, count,
                ) else {
                    return None;
                };
                (Some(checked), slots)
            }
            (None, Some((_, slots))) if !slots.is_empty() => return None,
            (Some(_), None) => return None,
            _ => (None, &[][..]),
        };
        let mut flat = Flat::default();
        let frontier: Vec<Digest> = (0..setting.metadata.len())
            .map(|rule| {
                frontier_flat(
                    &moved,
                    observations,
                    slots,
                    setting,
                    RuleIndex(rule),
                    &mut flat,
                );
                let digest = if flat.0.is_empty() {
                    Digest([0; 16])
                } else {
                    self.digest.digest_words(DigestWords(&flat.0))
                };
                flat.0.clear();
                digest
            })
            .collect();
        let noted = self.digest.digest(DigestBytes(&interface_bytes(tables).0));
        let mut shown = vec![noted];
        shown.extend(frontier.iter().copied());
        let mut ran = made_ran(tables, number, self.digest.combine(&shown), moved, base);
        ran.noted = noted;
        ran.frontier = frontier
            .iter()
            .enumerate()
            .filter(|(_, digest)| **digest != Digest([0; 16]))
            .map(|(rule, digest)| RuleFrontier {
                rule: PlacedRule(u32::try_from(rule).unwrap_or(u32::MAX)),
                digest: *digest,
            })
            .collect();
        Some(ran)
    }

    /// What the languages make of the project the manifests and the links between the files make.
    pub(crate) fn project_now(
        &self,
        input: &ProjectToRead<'_>,
        symbols: &mut Symbols,
        facts: &mut dyn FactsWorker,
    ) -> Result<Projected, FileAnalysisError> {
        let mut candidates = self.language.project_entries(input);
        candidates.sort_unstable();
        candidates.dedup();
        let observed_entries = ryo_language::domain::project_context::service::observed(
            candidates
                .iter()
                .map(|path| self.corpus.observe_entry(path))
                .collect(),
        );
        let input = ProjectToRead {
            observed_entries: &observed_entries,
            ..*input
        };
        let mut writer = facts.columns_mut()?;
        let answer = self.language.project(&input, symbols, &mut writer)?;
        if answer.observed_entries.as_slice()
            != ryo_language::domain::project_context::service::entries(&observed_entries)
        {
            return Err(FileAnalysisError::ProjectObservationMismatch);
        }
        let said: Vec<ReadInPart> = std::mem::take(&mut writer.partials);
        let mut projected = Projected {
            unknowns: Vec::new(),
            partials: MetadataUseCaseImpl.partial_of(&said, Recovery::Whole, &[]),
            refused: answer
                .refused
                .into_iter()
                .map(|each| ManifestRefused {
                    language: each.language,
                    path: each.path,
                    reason: each.reason,
                })
                .collect(),
            unlisted: answer.unlisted,
            observed_entries: ryo_language::domain::project_context::service::receipt(
                &observed_entries,
            ),
            ..Projected::default()
        };
        self.consulted(&answer.consulted, &mut projected)?;
        Ok(projected)
    }

    /// The files the lookup read beyond the manifests, each stat'ed, read and digested as the files of the run are, in the order of
    /// `paths`, and apart from them the ones that are not there, into `into`. An unrecordable dependency fails the run.
    pub(super) fn consulted(
        &self,
        paths: &[FilePath],
        into: &mut Projected,
    ) -> Result<(), FileAnalysisError> {
        let mut bytes = FileBytes::default();
        for path in paths {
            let entry = match self.corpus.stat(path) {
                Ok(PathStat::Found(entry)) => entry,
                // A file the lookup asked for and found none of is kept as such: one that appears changes what the lookup finds.
                Ok(PathStat::Missing) => {
                    into.consulted_absent.push(path.clone());
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            self.corpus.read(&entry, &mut bytes)?;
            into.consulted.push(StoredFile {
                executable: entry.executable,
                digest: self.digest.digest(DigestBytes(&bytes.0)),
                interface: Digest([0; 16]),
                noted: Digest([0; 16]),
                size: entry.size,
                modified: entry.modified,
                path: entry.path,
            });
        }
        Ok(())
    }
}

use ryo_execution::inbound::use_cases::metadata::interface::Governing;
