//! What the languages make of the project: its manifests, the project made now and the files its lookup consulted.
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::super::streaming::service::FileVisits;
use super::super::types::ProjectMaking;
use crate::domain::project::types::Projected;
use crate::domain::workers::service::note_slots;
use crate::domain::workers::types::{FileTotal, FreshNotes, ProjectHandle};
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::types::PathStat;
use crate::outbound::language::types::ProjectToRead;
use crate::outbound::language::types::{ManifestReading, ProjectNotes};
use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::types::{JobCount, JobNumber};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase as _;
use ryo_execution::inbound::use_cases::metadata::interface::Recovery;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution_runtime::outbound::digest::types::DigestBytes;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::service::input_path;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::runs::types::ManifestRefused;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stored_facts::types::ReadInPart;
use ryo_values::domain::stores::types::StoredFile;

impl FileAnalysis {
    /// Reads primary inventoried manifests once before project preparation, retaining genuine read failures.
    /// Consulted project dependencies use their separate lookup route and never become primary failure events.
    pub(crate) fn read_manifests(
        &self,
        files: &mut [CorpusInput],
    ) -> Result<
        Vec<ryo_language::inbound::use_cases::project::types::ManifestFile>,
        FileAnalysisError,
    > {
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

    /// What the languages make of the project the manifests and the links between the files make.
    pub(crate) fn project_now(
        &self,
        input: &ProjectToRead<'_>,
        symbols: &mut Symbols,
        facts: &mut dyn FactsWorker,
    ) -> Result<Projected, FileAnalysisError> {
        let mut candidates = self.language.project_entries(input).0;
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
            partials: MetadataUseCaseImpl
                .partial_of(&said, Recovery::Whole, &[])
                .0,
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
    pub(in crate::application::file_analysis) fn consulted(
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

impl FileVisits {
    /// The project made now on the scheduler's pool, and what the rules that read it could not decide for its partly read
    /// relations: a primary manifest that could not be read is not a complete empty project. The outer refusal is the scheduler's,
    /// the inner one the project's.
    pub(crate) fn project(
        &self,
        run: &ProjectMaking<'_>,
        project: &mut ProjectHandle<Box<dyn FactsWorker>>,
        notes: &mut FreshNotes,
    ) -> Result<Result<Projected, FileAnalysisError>, SchedulerError> {
        // What the languages wrote of each file this run read, in the order of the files: every file is read when the project is made.
        let notes = ProjectNotes(note_slots(notes, FileTotal(run.files.len())));
        let input = ProjectToRead {
            root: &run.session.root,
            repository: run.session.repository,
            search_scope: &run.session.search_scope,
            given_inventory: &run.session.given_inventory,
            observed_entries: &Default::default(),
            manifests: run.manifests,
            files: run.files,
            notes: &notes,
            wanted: run.wanted,
        };
        let mut now = match self.projected(&input, project)? {
            Ok(now) => now,
            Err(error) => return Ok(Err(error)),
        };
        // This affects project facts, not the disk-failure diagnosis of unrelated files.
        for input in run.files {
            let detail = match input {
                CorpusInput::ReadFailed { reason, .. } | CorpusInput::Unreadable { reason, .. }
                    if self.analysis.language.reads_manifest(input_path(input))
                        == ManifestReading::Read =>
                {
                    reason
                }
                CorpusInput::Available(_)
                | CorpusInput::ReadFailed { .. }
                | CorpusInput::Unreadable { .. } => continue,
            };
            now.partials
                .extend(crate::domain::project::service::unread_manifest_partials(
                    run.wanted,
                    input_path(input),
                    detail,
                ));
        }
        // The files the rows of the project are about: those the run lists, then the manifests it does not.
        let named: Vec<FilePath> = run
            .files
            .iter()
            .map(|file| input_path(file).clone())
            .chain(now.unlisted.iter().cloned())
            .collect();
        now.unknowns = MetadataUseCaseImpl
            .project_unknowns(&now.partials, &named, run.needs, run.ids)
            .0;
        Ok(Ok(now))
    }

    /// What the project writes now, made from its manifests and the notes the files left, on the scheduler's pool, the one the edge
    /// built: whatever parallel work a language does inside it runs on that pool's workers, never on a pool of its own. The outer
    /// refusal is the scheduler's, the inner one the project's.
    fn projected(
        &self,
        input: &ProjectToRead<'_>,
        project: &mut ProjectHandle<Box<dyn FactsWorker>>,
    ) -> Result<Result<Projected, FileAnalysisError>, SchedulerError> {
        let mut projected = None;
        {
            let analysis = &self.analysis;
            let made = &mut projected;
            let mut job: Box<dyn FnMut(JobNumber) + Send + '_> = Box::new(move |_| {
                *made =
                    Some(analysis.project_now(input, &mut project.symbols, &mut *project.facts));
            });
            self.scheduler
                .run(std::slice::from_mut(&mut job), JobCount(1))?;
        }
        projected.ok_or(SchedulerError::NoWorkers)
    }
}

impl FileAnalysis {
    /// SPIKE (ryo-fmx3.1): the project's rows of the `wanted` relations of project scope, for the new engine's scans: made as
    /// today's run makes them (`project_now`'s reading, into a writer that keeps them), after a pass of today's reader over
    /// each file for its notes when a wanted relation is made from them. Nothing is made when no wanted relation is the
    /// project's. The notes pass reads every file once more: its cost is noted, not optimised.
    pub(crate) fn hosted_project(
        &self,
        files: &[CorpusInput],
        session: &ryo_values::domain::runs::types::Session,
        manifests: &[ryo_language::inbound::use_cases::project::types::ManifestFile],
        wanted: &[ryo_relations::domain::relations::types::Relation],
        source_language: Option<&ryo_values::domain::descriptions::types::LanguageName>,
    ) -> Result<
        ryo_execution_runtime::application::file_scan::hosted::types::HostedRows,
        FileAnalysisError,
    > {
        use ryo_execution_runtime::application::file_scan::hosted::service::kept;
        use ryo_execution_runtime::application::file_scan::hosted::types::{HostedRows, HostedWriter};
        use ryo_relations::domain::catalog::service::{needs_notes, scope_of};
        use ryo_relations::domain::catalog::types::{RelationNotes, RowScope};
        let held: Vec<_> = wanted
            .iter()
            .copied()
            .filter(|relation| scope_of(*relation) == RowScope::Project)
            .collect();
        if held.is_empty() {
            return Ok(HostedRows::default());
        }
        let noted = held
            .iter()
            .any(|relation| needs_notes(*relation) == RelationNotes::Needed);
        let notes = ProjectNotes(if noted {
            self.notes_of(files, source_language)?
        } else {
            vec![None; files.len()]
        });
        let input = ProjectToRead {
            root: &session.root,
            repository: session.repository,
            search_scope: &session.search_scope,
            given_inventory: &session.given_inventory,
            observed_entries: &Default::default(),
            manifests,
            files,
            notes: &notes,
            wanted,
        };
        let mut candidates = self.language.project_entries(&input).0;
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
            ..input
        };
        let (mut symbols, mut writer) = (Symbols::default(), HostedWriter::default());
        let _answer = self.language.project(&input, &mut symbols, &mut writer)?;
        Ok(kept(writer, &symbols, held))
    }

    /// SPIKE (ryo-fmx3.1): what today's reader writes of each file for the project, in the order of the files: none for a
    /// file that is not read.
    fn notes_of(
        &self,
        files: &[CorpusInput],
        source_language: Option<&ryo_values::domain::descriptions::types::LanguageName>,
    ) -> Result<Vec<Option<ryo_values::domain::stored_facts::types::FileNotes>>, FileAnalysisError> {
        use crate::outbound::language::types::{
            FileReading, FileToRead, NoteWriting, RoleWriting, SourceSelection,
        };
        use ryo_language::domain::elements::service::take_notes;
        use ryo_values::domain::stored_facts::types::{FileNotes, NoteBytes};
        let (mut bytes, mut table) = (
            FileBytes::default(),
            ryo_language::domain::elements::types::Elements::default(),
        );
        let mut notes = Vec::with_capacity(files.len());
        for (at, input) in files.iter().enumerate() {
            let CorpusInput::Available(entry) = input else {
                notes.push(None);
                continue;
            };
            bytes.0.clear();
            if self.corpus.read(entry, &mut bytes).is_err() {
                notes.push(None);
                continue;
            }
            let mut symbols = Symbols::default();
            let read = FileToRead {
                source_language: source_language
                    .map_or(SourceSelection::Recognized, SourceSelection::Named),
                path: &entry.path,
                file: ryo_relations::domain::relations::types::FileId(
                    u32::try_from(at).unwrap_or(u32::MAX),
                ),
                bytes: &bytes,
                wanted: &[],
                notes: NoteWriting::Wanted,
                roles: RoleWriting::Unwanted,
            };
            let FileReading::Read(language) = self.language.parse(read, &mut table, &mut symbols)
            else {
                notes.push(None);
                continue;
            };
            let written = take_notes(&mut table).0;
            notes.push((!written.is_empty()).then(|| FileNotes {
                language,
                notes: NoteBytes(written),
            }));
        }
        Ok(notes)
    }
}
