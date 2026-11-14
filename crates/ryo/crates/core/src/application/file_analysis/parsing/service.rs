//! The read of a file the run has not kept: the language parses it, and the run reads what it wrote and the engine's own rows.
use super::super::errors::FileAnalysisError;
use super::super::semantic::service::SemanticState;
use super::super::service::FileAnalysis;
use super::super::types::{EngineFile, FileCompleteness, FileRead, FileVisit, Worker};
use crate::application::file_analysis::constants::EXECUTION_DIAGNOSTIC;
use crate::application::file_analysis::constants::PROVIDER_PARSE;
use crate::domain::reading::service::begin;
use crate::domain::reading::service::engine_asked;
use crate::domain::reading::service::failure_partials;
use crate::domain::reading::service::prepare;
use crate::domain::reading::service::start_partials;
use crate::domain::reading::types::EngineAsked;
use crate::domain::reading::types::EngineRows;
use crate::domain::reading::types::FailureDetail;
use crate::domain::reading::types::ObservedFile;
use crate::domain::reading::types::PrimaryFailureKind;
use crate::domain::recording::service::cover;
use crate::domain::render::types::Answering;
use crate::domain::workers::types::Gating;
use crate::outbound::language::types::FileReading;
use crate::outbound::language::types::FileToRead;
use crate::outbound::language::types::SourceSelection;
use ryo_execution::inbound::use_cases::execution::interface::ProgramPreparedExecutionPort as _;
use ryo_execution::inbound::use_cases::execution::interface::SemanticNeed;
use ryo_execution::inbound::use_cases::metadata::interface::Recovery;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::diagnostics::types::SyntaxCompleteness;
use ryo_language::domain::elements::service::has_incomplete_lexical_stream;
use ryo_language::domain::elements::service::has_incomplete_syntax;
use ryo_language::domain::elements::service::syntax_acceptance;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::symbols::service::clear_symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::ExecutablePermission;
use ryo_values::domain::records::types::FileOutcome;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::records::types::Unreadable;
use ryo_values::domain::stored_facts::types::ReadFailure;
use ryo_values::domain::stored_facts::types::ReadInPart;
use std::time::Instant;

impl FileAnalysis {
    /// Executes demanded engine facts for a primary input failure without constructing source or syntax.
    pub(in crate::application::file_analysis) fn read_failed(
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
        let setting = &visit.setting;
        worker.tables.bytes.0.clear();
        prepare(&mut worker.tables);
        clear_symbols(&mut worker.tables.symbols);
        worker.tables.language = None;
        worker.facts.release()?;
        let asked = engine_asked(setting.wanted);
        let said = failure_partials(setting.wanted, number, detail, observed_file);
        worker.tables.unread.push(Unread {
            path: path.clone(),
            reason: Unreadable::Failed(ReadFailure(detail.0.into())),
        });
        let acceptance = syntax_acceptance(&worker.tables.elements);
        if asked.any == EngineRows::Asked {
            // The file runs, so its partiality is the run's.
            index_kinds(&mut worker.tables.elements);
            let semantic = SemanticState::new(number, &worker.tables.elements, None);
            Self::read_file(
                worker,
                visit,
                number,
                &semantic,
                FileRead {
                    language: &*self.language,
                    path,
                    said: &said,
                    recovery: Recovery::Whole,
                    active: setting.wanted,
                    engine: EngineFile {
                        asked: EngineAsked {
                            file: if observed_file == ObservedFile::Observed {
                                asked.file
                            } else {
                                EngineRows::NotAsked
                            },
                            lines: EngineRows::NotAsked,
                            ..asked
                        },
                        executable,
                        failure: (kind == PrimaryFailureKind::Disk).then_some(detail),
                    },
                },
            )?;
        } else {
            // No rule runs on the file: its partiality is core's to note.
            start_partials(
                &MetadataUseCaseImpl,
                &mut worker.tables,
                &said,
                Recovery::Whole,
                setting.wanted,
            );
            if Self::may_gate_rules(setting, acceptance) == Gating::MayGate {
                drop(Self::gate_rules(
                    &mut worker.tables,
                    setting,
                    number,
                    acceptance,
                ));
            }
        }
        cover(
            &MetadataUseCaseImpl,
            &mut worker.tables,
            setting,
            number,
            path,
            acceptance,
        );
        // Failure facts are invocation-local: retry the real input next time even if metadata is unchanged.
        if matches!(setting.answering, Answering::Findings) {
            worker.tables.unrecorded.push(path.clone());
        }
        Ok(())
    }

    /// The read of a file the run has not kept: the language parses it, and the per-file steps run on its rows, which the run's
    /// fact sources write and admit. Answers what the store keeps of it.
    pub(in crate::application::file_analysis) fn parse_file(
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
        let parse_started = tracing::enabled!(target: EXECUTION_DIAGNOSTIC, tracing::Level::TRACE)
            .then(Instant::now);
        let reading = self.language.parse(
            source,
            &mut worker.tables.elements,
            &mut worker.tables.symbols,
        );
        // A parse longer than u64 nanoseconds (five centuries) is not traced: it has no elapsed_ns to give.
        if let Some(started) = parse_started
            && let Ok(elapsed_ns) = u64::try_from(started.elapsed().as_nanos())
        {
            tracing::trace!(target: EXECUTION_DIAGNOSTIC, phase = PROVIDER_PARSE, file = number.0, elapsed_ns);
        }
        // What the readers and the engine's own rows say is read in part is the run's: they are read inside it. Core says nothing.
        let said: &[ReadInPart] = &[];
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
        let asked = engine_asked(active_wanted.as_ref());
        // The relations this file is read in part: those a reader said so of, and all of code if what was read is a recovery.
        let everything = recovery_of(&reading, &worker.tables.elements);
        let semantic_needed = setting.program.semantic_needed();
        let session = if semantic_needed == SemanticNeed::Required {
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
                    .begin_semantics(language, source, &mut worker.tables.elements)
                    .into()
            })
        } else {
            None
        };
        let semantic = SemanticState::new(number, &worker.tables.elements, session);
        let read = FileRead {
            language: &*self.language,
            path: &file.path,
            said,
            recovery: everything,
            active: active_wanted.as_ref(),
            engine: EngineFile {
                asked,
                executable: file.executable,
                failure: None,
            },
        };
        Ok(match reading {
            FileReading::Read(_) | FileReading::Manifest => {
                index_kinds(&mut worker.tables.elements);
                Self::read_file(worker, visit, number, &semantic, read)?;
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
                    Self::read_file(worker, visit, number, &semantic, read)?;
                } else {
                    // No rule runs on the file: its partiality is core's to note.
                    start_partials(
                        &MetadataUseCaseImpl,
                        &mut worker.tables,
                        said,
                        everything,
                        active_wanted.as_ref(),
                    );
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
}

/// Whether the language read a complete tree or a recovery from missing syntax.
pub(in crate::application::file_analysis) fn recovery_of(
    reading: &FileReading,
    elements: &Elements,
) -> Recovery {
    match reading {
        FileReading::Read(_)
            if has_incomplete_syntax(elements) == SyntaxCompleteness::Incomplete =>
        {
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
