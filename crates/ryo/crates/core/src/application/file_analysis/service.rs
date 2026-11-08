//! Bounded read, parse and per-file evaluation of one file in the worker that took its job, over injected ports; its parts read,
//! gate, evaluate, replay and record the file.
use crate::domain::budget::service::within;
use crate::domain::reading::service::read_failure_demanded;
use crate::domain::reading::types::FailureDetail;
use crate::domain::reading::types::ObservedFile;
use crate::domain::reading::types::PrimaryFailureKind;
use crate::domain::reading::types::ReadFailureDemand;
use crate::domain::reading::types::RunRuleCount;
use crate::domain::recording::service::cover;
use crate::domain::recording::service::finish_file;
use crate::domain::handing::service::replay;
use crate::domain::render::types::Answering;
use crate::domain::run_rules::service::governing;
use crate::domain::visit::types::Keeping;
use crate::domain::workers::service::checking;
use crate::domain::workers::service::file_at;
use crate::domain::workers::service::govern;
use crate::domain::workers::types::Checking;
use crate::domain::workers::types::FileOrder;
use crate::domain::workers::types::Tables;
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::memory::interface::MemoryPort;
use crate::outbound::scheduler::types::JobNumber;
use ryo_execution::inbound::use_cases::metadata::interface::Governing;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_execution_runtime::outbound::digest::types::DigestBytes;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_language::domain::elements::service::syntax_acceptance;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::files::service::input_path;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::files::types::ExecutablePermission;
use ryo_values::domain::files::types::FileNumber;
use ryo_values::domain::records::service::is_as_it_was;
use ryo_values::domain::records::types::Freshness;
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;
use super::errors::FileAnalysisError;
use super::types::{FileVisit, Worker};


/// Analysis of one file or the project, with resources supplied by the composition.
pub struct FileAnalysis {
    /// Reads each file's bytes.
    pub(in crate::application::file_analysis) corpus: Arc<dyn CorpusPort>,
    /// Parses a file into its syntax tree and rows, and names the languages linked.
    pub(in crate::application::file_analysis) language: Arc<dyn LanguagePort>,
    /// Digests the files whose results the store keeps between runs.
    pub(in crate::application::file_analysis) digest: Arc<dyn DigestPort>,
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
        if govern(&MetadataUseCaseImpl, setting, &mut worker.tables, input_path(file)) == Governing::DoesNotGovern {
            return Ok(());
        }
        if checking(at) == Checking::Now {
            let mask = &worker.tables.mask;
            within(self.memory.resident_bytes()?, setting.budget, || {
                governing(&MetadataUseCaseImpl, mask, setting.ids)
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
            read_failure_demanded(&MetadataUseCaseImpl, 
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
        cover(&MetadataUseCaseImpl, &mut worker.tables, setting, number, &file.path, acceptance);
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
}
