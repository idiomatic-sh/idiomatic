//! The lints of the cases: one over the fakes, scheduled serially or in parallel, and again over one store.
use super::super::errors::LintError;
use super::super::interface::LintUseCase as _;
use super::super::use_case::LintUseCaseImpl;
use super::given::Given;
use super::given::Lint;
use super::given::SOURCES;
use super::given::World;
use super::given::request;
use crate::cases::run;
use crate::inbound::use_cases::lint::fixtures::given::violations;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use crate::outbound::row_sink;
use crate::outbound::row_sink::fixtures::fake::Ends;
use crate::outbound::row_sink::fixtures::fake::RowSinkFake;
use crate::outbound::row_sink::interface::RowSinkPort;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::WorkerCount;
use ryo_execution::outbound::matching::fixtures::fake::MatchingFake;
use ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::CorpusFile;
use ryo_values::domain::files::types::FileBytes;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// The use case over the fakes `given` stands for.
pub(crate) fn world(mut given: Given) -> World {
    let scheduler = Arc::new(std::mem::take(&mut given.scheduler));
    let query_language: Arc<
        dyn ryo_execution::outbound::query_language::interface::QueryLanguagePort,
    > = Arc::<QueryLanguageFake>::clone(&given.query);
    scheduled_world_with_query(given, scheduler, WorkerCount(1), query_language)
}

/// The same fixture over an explicitly injected production scheduler.
pub(crate) fn scheduled_world(
    given: Given,
    scheduler: Arc<dyn SchedulerPort>,
    workers: WorkerCount,
) -> World {
    let query_language: Arc<
        dyn ryo_execution::outbound::query_language::interface::QueryLanguagePort,
    > = Arc::<QueryLanguageFake>::clone(&given.query);
    scheduled_world_with_query(given, scheduler, workers, query_language)
}

/// The same test world with an explicitly supplied query compiler.
pub(crate) fn scheduled_world_with_query(
    given: Given,
    scheduler: Arc<dyn SchedulerPort>,
    workers: WorkerCount,
    query_language: Arc<dyn ryo_execution::outbound::query_language::interface::QueryLanguagePort>,
) -> World {
    let memory = Arc::new(given.memory);
    let operators = Arc::new(
        ryo_execution_runtime::outbound::operators::factory::use_case(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
                std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()),
            ),
        ),
    );
    scheduled_world_with_query_resources(
        given,
        scheduler,
        workers,
        query_language,
        memory,
        operators,
    )
}

pub(crate) fn scheduled_world_with_query_resources(
    given: Given,
    scheduler: Arc<dyn SchedulerPort>,
    workers: WorkerCount,
    query_language: Arc<dyn ryo_execution::outbound::query_language::interface::QueryLanguagePort>,
    analysis_memory: Arc<dyn crate::outbound::memory::interface::MemoryPort>,
    operators: Arc<dyn ryo_execution_runtime::outbound::operators::interface::OperatorsPort>,
) -> World {
    let collected = row_sink::factory::CollectorConnection::default();
    let kept: Arc<dyn RowSinkPort> = Arc::new(row_sink::factory::collect(collected.clone()));
    let ports: Arc<dyn RowSinkPort> = if given.ends == Ends::Well {
        kept
    } else {
        Arc::new(RowSinkFake {
            ends: given.ends,
            ..RowSinkFake::default()
        })
    };
    let language = Arc::new(given.language);
    let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let corpus_file = |(path, text): &(&str, &str)| CorpusFile {
        executable: Default::default(),
        path: FilePath((*path).into()),
        bytes: FileBytes(text.as_bytes().to_vec()),
    };
    let files = given.files.iter().map(corpus_file);
    let outside = given
        .outside
        .iter()
        .map(corpus_file)
        .map(|mut file| {
            file.executable = given.outside_executable;
            file
        })
        .collect();
    let store: Arc<dyn crate::outbound::lint_store::interface::LintStorePort> = match given.kept_in
    {
        Some(other) => other,
        None => given.store,
    };
    let use_case = {
        let analysis_corpus: Arc<dyn crate::outbound::corpus::interface::CorpusPort> =
            Arc::new(CorpusFake {
                files: files.collect(),
                fails: given.corpus_fails,
                reads: Arc::clone(&reads),
                outside,
            });
        let analysis_language: Arc<dyn crate::outbound::language::interface::LanguagePort> =
            Arc::<LanguageFake>::clone(&language);
        let analysis_digest: Arc<
            dyn ryo_execution_runtime::outbound::digest::interface::DigestPort,
        > = Arc::new(DigestFake);
        LintUseCaseImpl::new(
            crate::application::file_analysis::service::FileAnalysis::new(
                Arc::clone(&analysis_corpus),
                Arc::clone(&analysis_language),
                Arc::clone(&analysis_digest),
                Arc::clone(&analysis_memory),
            ),
            crate::application::cross_file::service::CrossFile::new(Arc::clone(&analysis_corpus), std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())),
            Arc::clone(&analysis_corpus),
            Arc::new(given.facts),
            Arc::clone(&analysis_language),
            Arc::new(MatchingFake),
            query_language,
            Arc::clone(&analysis_memory),
            operators,
            ryo_execution::application::file_dag::derived::service::derived(),
            scheduler,
            workers,
            ports,
            Arc::clone(&analysis_digest),
            store,
        )
    };
    World {
        use_case,
        collected,
        language,
        query: given.query,
        reads,
    }
}

/// One lint over what `given` stands for.
pub(crate) fn lint(given: Given) -> Lint {
    let asked = request(&given);
    let world = world(given);
    let report = world.use_case.lint(asked);
    Lint {
        report: report.map(|answer| answer.unread.len()),
        rows: std::mem::take(
            &mut *world
                .collected
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
        .rows,
        parses: world.language.parses.load(Ordering::SeqCst),
        projects: world.language.projects.load(Ordering::SeqCst),
        compiles: world.query.calls.load(Ordering::SeqCst),
        reads: world.reads.load(Ordering::SeqCst),
    }
}

/// Complete ordered answers agree across worker counts, including reused and edited stores.
pub(crate) fn serial_and_parallel_agree() {
    let fresh = || Arc::new(LintStoreFake::default());
    let serial = fresh();
    let parallel = fresh();
    let edited = [("a.rs", "fn changed() {}"), SOURCES[1], SOURCES[2]];
    for files in [&SOURCES, &SOURCES, &edited] {
        compare_schedulers(files, &serial, &parallel, false, Ends::Well);
    }
    compare_schedulers(&SOURCES, &fresh(), &fresh(), true, Ends::Well);
    compare_schedulers(&SOURCES, &fresh(), &fresh(), false, Ends::Failed);
}

/// Compare every report field and the complete ordered output, rather than row counts.
pub(crate) fn compare_schedulers(
    files: &[(&'static str, &'static str)],
    serial: &Arc<LintStoreFake>,
    parallel: &Arc<LintStoreFake>,
    corpus_fails: bool,
    ends: Ends,
) {
    let execute =
        |store: &Arc<LintStoreFake>, scheduler: Arc<dyn SchedulerPort>, workers: WorkerCount| {
            let given = Given {
                files: files.to_vec(),
                store: Arc::clone(store),
                corpus_fails,
                ends,
                ..violations()
            };
            let asked = request(&given);
            let world = scheduled_world(given, scheduler, workers);
            (
                world.use_case.lint(asked),
                std::mem::take(
                    &mut *world
                        .collected
                        .0
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner),
                ),
            )
        };
    assert_eq!(
        execute(
            serial,
            Arc::new(crate::outbound::scheduler::factory::sequential()),
            WorkerCount(1)
        ),
        execute(
            parallel,
            Arc::new(crate::outbound::scheduler::factory::rayon(WorkerCount(2))),
            WorkerCount(2)
        ),
        "worker scheduling preserves every report field, column, row and output summary"
    );
}

/// The files read and the files left unread by a lint.
pub(crate) fn read_and_unread(run: Lint) -> Result<(usize, usize), LintError> {
    run.report.map(|unread| (run.parses, unread))
}

/// A lint of `files` at `root` with `rule` as the rule's text, over `store`.
pub(crate) fn again(
    files: &[(&'static str, &'static str)],
    root: &'static str,
    rule: &'static str,
    store: &Arc<LintStoreFake>,
) -> Lint {
    lint(Given {
        files: files.to_vec(),
        store: Arc::clone(store),
        root,
        rules: vec![rule],
        ..violations()
    })
}
