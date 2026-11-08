//! The lint counting what it asks of the language, over any root: what the suites that measure a run share.

use super::base::{Pending, recordings, rule_of};
use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
use std::path::Path;
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::{LintRequest, Rule};
use ryo_core::inbound::use_cases::lint::use_case::LintUseCaseImpl;
use ryo_core::outbound::language::factory::LanguageConnection;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::runs::types::Budget;
use ryo_values::domain::runs::types::Session;

use ryo_core::outbound::row_sink::interface::RowSinkPort;
use ryo_core::outbound::{corpus, language, memory, row_sink, scheduler};
use ryo_execution::outbound::{digest, facts, matching, query_language};
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;

/// The language, counting the files it was asked to read.
pub(crate) struct Counting {
    /// The language every call is handed on to.
    pub language: Arc<dyn language::interface::LanguagePort>,
    /// How many files the language was asked to read, shared with the caller that reads it after the run.
    pub reads: Arc<std::sync::atomic::AtomicUsize>,
    /// How many times the language was asked to read the project, shared as `reads` is.
    pub projects: Arc<std::sync::atomic::AtomicUsize>,
}

impl language::interface::LanguagePort for Counting {
    fn relations(&self) -> Result<language::types::RelationList, language::errors::LanguageError> {
        self.language.relations()
    }

    fn linked(&self) -> language::types::LinkedNames {
        self.language.linked()
    }

    fn unknown_reasons(&self) -> language::types::UnknownReasons {
        self.language.unknown_reasons()
    }

    fn kind_vocabularies(&self) -> ryo_values::domain::descriptions::types::KindVocabularies {
        self.language.kind_vocabularies()
    }

    fn supports(
        &self,
        relation: ryo_relations::domain::relations::types::Relation,
    ) -> language::types::Answers {
        self.language.supports(relation)
    }

    fn reads_manifest(
        &self,
        path: &ryo_language::domain::files::types::FilePath,
    ) -> language::types::ManifestReading {
        self.language.reads_manifest(path)
    }

    fn parse(
        &self,
        file: language::types::FileToRead<'_>,
        table: &mut ryo_language::domain::elements::types::Elements,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) -> language::types::FileReading {
        let _before = self
            .reads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.language.parse(file, table, symbols, out)
    }

    fn project_entries(
        &self,
        input: &language::types::ProjectToRead<'_>,
    ) -> Vec<ryo_language::domain::files::types::FilePath> {
        self.language.project_entries(input)
    }

    fn project(
        &self,
        input: &language::types::ProjectToRead<'_>,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) -> Result<language::types::ProjectAnswer, language::errors::LanguageError> {
        let _before = self
            .projects
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.language.project(input, symbols, out)
    }
}

/// What a lint gave and what it asked of the language, counted: the rows, the files nothing read, and how many files were read.
/// A test that needs the run's complete output and report observes them through [`lint_observing`].
pub(crate) struct Counted {
    /// The rows of the findings.
    pub rows: Vec<Vec<String>>,
    /// The files nothing read.
    pub unread: Vec<String>,
    /// How many files the language was asked to read.
    pub reads: usize,
    /// How many times the language was asked to read the project.
    pub projects: usize,
}

/// Lints `root` with `rules`, keeping what it found in `lint_store`, and counts the files the language read.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_counting(
    root: &Path,
    rules: Vec<Rule>,
    lint_store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort>,
) -> Result<Counted, String> {
    lint_with_workers(root, rules, lint_store, 0)
}

/// Lints `root` with `rules` on `workers` workers (none: as many as the machine has), keeping what it found in `lint_store`, and counts.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_with_workers(
    root: &Path,
    rules: Vec<Rule>,
    lint_store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort>,
    workers: usize,
) -> Result<Counted, String> {
    lint_observing(
        root,
        rules,
        lint_store,
        workers,
        |counted, _output, _report| counted,
    )
}

/// Lints `root` with `rules` on `workers` workers, keeping what it found in `lint_store`, and gives `observe` what the run counted with its
/// complete emitted output and report: each test keeps what it reads of them.
///
/// # Errors
///
/// What the lint refuses, as text.
pub(crate) fn lint_observing<T>(
    root: &Path,
    rules: Vec<Rule>,
    lint_store: Arc<dyn ryo_core::outbound::lint_store::interface::LintStorePort>,
    workers: usize,
    observe: impl FnOnce(
        Counted,
        row_sink::types::Collected,
        ryo_core::inbound::use_cases::lint::types::LintReport,
    ) -> T,
) -> Result<T, String> {
    let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let projects = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let rust = ryo_rust::linked();
    let compile = CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    );
    let collected = row_sink::factory::CollectorConnection::default();
    let kept: Arc<dyn RowSinkPort> = Arc::new(row_sink::factory::collect(collected.clone()));
    let use_case = {
        let analysis_corpus: Arc<dyn corpus::interface::CorpusPort> =
            Arc::new(corpus::factory::ignore(
                RootPath(root.to_string_lossy().into()),
                ryo_values::domain::files::types::CorpusListing::Walked,
                Vec::new(),
                corpus::types::SkippedDirectories(vec![".ryo_cache".into()]),
            ));
        let analysis_language: Arc<dyn language::interface::LanguagePort> = Arc::new(Counting {
            language: Arc::new(language::factory::use_case(vec![LanguageConnection::from(
                rust,
            )])),
            reads: Arc::clone(&reads),
            projects: Arc::clone(&projects),
        });
        let analysis_memory: Arc<dyn memory::interface::MemoryPort> =
            Arc::new(memory::factory::sysinfo(Budget::Machine));
        let analysis_digest: Arc<
            dyn ryo_execution::outbound::digest::interface::DigestPort,
        > = Arc::new(digest::factory::blake3());
        let started = scheduler::types::WorkerCount(
            ryo_values::domain::runs::service::workers_to_start(
                ryo_values::domain::runs::types::WorkersAsked(workers),
                ryo_values::domain::runs::types::MachineWorkers(
                    u32::try_from(std::thread::available_parallelism().map_or(1, std::num::NonZero::get)).unwrap_or(1),
                ),
            )
            .0,
        );
        LintUseCaseImpl::new(
            ryo_core::application::file_analysis::service::FileAnalysis::new(
                Arc::clone(&analysis_corpus),
                Arc::clone(&analysis_language),
                Arc::clone(&analysis_digest),
                Arc::clone(&analysis_memory),
            ),
            ryo_core::application::cross_file::service::CrossFile::new(Arc::clone(
                &analysis_corpus,
            ), std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())),
            Arc::clone(&analysis_corpus),
            Arc::new(facts::factory::memory()),
            Arc::clone(&analysis_language),
            Arc::new(matching::factory::globset()),
            Arc::new(query_language::factory::use_case(
                Arc::new(compile),
                Arc::new(TypecheckUseCaseImpl::new(
                    Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
                    Arc::new(ryo_query_language::outbound::catalog::factory::language()),
                )),
            )),
            Arc::clone(&analysis_memory),
            Arc::new(ryo_execution::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
            )),
            Arc::new(ryo_execution_store::outbound::derived::factory::use_case(
                ryo_execution_store::outbound::derived::factory::semantics(),
            )),
            Arc::new(scheduler::factory::rayon(started)),
            started,
            Arc::clone(&kept),
            Arc::clone(&analysis_digest),
            lint_store,
        )
    };
    let request = LintRequest {
        source_language: ryo_core::inbound::use_cases::lint::types::SourceProvider::Recognized,
        answer: ryo_values::domain::runs::types::AnswerShape::Findings,
        hosted: Vec::new(),
        session: Session {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
            root: RootPath(root.to_string_lossy().into()),
        },
        files: Vec::new(),
        rules,
    };
    let report = use_case.lint(request).map_err(|error| error.to_string())?;
    let output = std::mem::take(
        &mut *collected
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    let rows = output
        .rows
        .iter()
        .map(|row| crate::base::finding_texts(&row.0))
        .collect();
    let counted = Counted {
        rows,
        unread: report
            .unread
            .iter()
            .map(|file| file.path.0.to_string())
            .collect(),
        reads: reads.load(std::sync::atomic::Ordering::Relaxed),
        projects: projects.load(std::sync::atomic::Ordering::Relaxed),
    };
    Ok(observe(counted, output, report))
}

/// The rules of the recordings that run today, by name: what the bench measures over ruff.
///
/// # Panics
///
/// When the recordings cannot be read.
pub(crate) fn runnable() -> Vec<(String, Rule)> {
    recordings()
        .iter()
        .filter(|each| each.pending == Pending::Missing)
        .map(|each| (each.name.clone(), rule_of(each)))
        .collect()
}
