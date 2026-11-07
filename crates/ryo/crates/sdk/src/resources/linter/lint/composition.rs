//! The wiring of `linter lint`: the edge takes the parts its caller supplies, chooses each implementation
//! in its builders, and builds the service it calls. `compose` only constructs. Nothing here implements a port.

use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::interface::LintUseCase;
use ryo_core::inbound::use_cases::open::interface::OpenUseCase;
use ryo_core::outbound::lint_store::interface::LintStorePort;
use ryo_core::outbound::lint_store::types::BuildIdentity;
use ryo_core::outbound::{corpus, language, lint_store, memory, row_sink, scheduler};
use ryo_execution::outbound::{digest, facts, matching, query_language};
use ryo_execution::outbound::matching::interface::MatchingPort;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusFile, CorpusListing, FileBytes};

use crate::domain::capacity::types::{MemoryBudget, WorkerPool};
use crate::domain::language::types::Language;

use super::connections::LinterLintConnections;
use super::types::{Corpus, LintRequest, LinterLintEngine, Reading, Spill};
use crate::domain::project::types::{DependencyBytes, DependencyFingerprint};

/// The dependencies the caller supplies: how the service is built over the adapters its builder chose.
pub(crate) struct LinterLintParts<Construct> {
    /// Builds the service.
    pub service: Construct,
}

/// Wires the service from the supplied dependencies; it only constructs.
pub(crate) fn compose<Service, Construct>(parts: LinterLintParts<Construct>) -> Service
where
    Service: LintUseCase,
    Construct: FnOnce() -> Service,
{
    (parts.service)()
}

/// The production place the rows of a run are kept in, which the caller takes them from after the run.
pub(crate) fn build_rows() -> row_sink::factory::CollectorConnection {
    row_sink::factory::CollectorConnection::default()
}

/// What the caller set for one run's adapters: the repository the corpus reads and the memory the run may use. The pool
/// the scheduler runs on is the linter's, injected once when it was built.
pub(crate) struct LinterLintSettings {
    /// The repository's root directory.
    pub root: ryo_language::inbound::use_cases::project::types::RootPath,
    /// The memory the run may use.
    pub budget: ryo_values::domain::runs::types::Budget,
}

/// Chooses the parts every run over a linter shares, once: the languages linked, the compiler of queries and its
/// catalog, and the matching.
pub(crate) fn build_engine(
    languages: Vec<Language>,
    pool: WorkerPool,
    build: BuildIdentity,
    memory: MemoryBudget,
    cache: super::types::Cache,
) -> LinterLintEngine {
    let workers = pool.workers();
    let skipped = corpus::types::SkippedDirectories(
        languages
            .iter()
            .flat_map(|language| language.skips().iter().map(|skip| skip.0.into()))
            .collect(),
    );
    let handles = languages
        .into_iter()
        .map(language::factory::LanguageConnection::from)
        .collect();
    let syntax = Arc::new(ryo_query_language::outbound::syntax::factory::winnow());
    let catalog = Arc::new(ryo_query_language::outbound::catalog::factory::language());
    let compile = Arc::new(
        ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl::new(
            syntax, catalog,
        ),
    );
    let typecheck = Arc::new(
        ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
            Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
            Arc::new(ryo_query_language::outbound::catalog::factory::language()),
        ),
    );
    LinterLintEngine {
        skipped,
        language: Arc::new(language::factory::use_case(handles)),
        query_language: Arc::new(query_language::factory::use_case(compile, typecheck)),
        matching: Arc::new(matching::factory::globset()),
        plans: ryo_execution::application::rule_compilation::service::CompiledPlans::default(),
        scheduler: pool.of(),
        workers,
        build,
        memory,
        cache,
    }
}

/// Chooses the production adapters of one run over `engine` and hands `compose` the construction of the service over
/// them: the store in the cache directory the connections name, or one in memory for a run that keeps nothing.
pub(crate) fn build(
    engine: &LinterLintEngine,
    collector: row_sink::factory::CollectorConnection,
    reading: Reading,
    connections: &LinterLintConnections,
    settings: LinterLintSettings,
) -> impl LintUseCase {
    let LinterLintSettings { root, budget } = settings;
    let spill: Arc<dyn ryo_execution::outbound::facts::interface::FactsPort> =
        match &connections.spill {
            Spill::Kept(path) => Arc::new(facts::factory::spill(path.clone())),
            Spill::Scratch => Arc::new(facts::factory::spill_scratch()),
        };
    let store: Arc<dyn LintStorePort> = match connections.store.clone() {
        Some(store) => Arc::new(lint_store::factory::postcard(store)),
        None => Arc::new(lint_store::factory::discard()),
    };
    // The walk does not go into the directory that holds the store, when the store is kept beneath the root.
    let mut skipped = engine.skipped.clone();
    skipped.0.extend(
        connections
            .store
            .iter()
            .filter_map(|kept| {
                store_skip(
                    std::path::Path::new(&*kept.directory.0),
                    std::path::Path::new(&*root.0),
                )
            })
            .filter_map(|skip| skip.to_str().map(Box::<str>::from)),
    );
    let analysis_language = Arc::clone(&engine.language);
    let query = Arc::clone(&engine.query_language);
    let matching = Arc::clone(&engine.matching);
    let plans = engine.plans.clone();
    let pool = Arc::clone(&engine.scheduler);
    let workers = engine.workers;
    compose(LinterLintParts {
        service: move || {
            let files: Arc<dyn corpus::interface::CorpusPort> = match reading {
                Reading::Disk(listing, beside) => {
                    Arc::new(corpus::factory::ignore(root, listing, beside, skipped))
                }
                Reading::Memory(files, inventory) => {
                    Arc::new(corpus::factory::memory(files, inventory))
                }
            };
            let analysis_corpus: Arc<dyn corpus::interface::CorpusPort> = files;
            let analysis_memory: Arc<dyn memory::interface::MemoryPort> =
                Arc::new(memory::factory::sysinfo(budget));
            let analysis_digest: Arc<
                dyn ryo_execution::outbound::digest::interface::DigestPort,
            > = Arc::new(digest::factory::blake3());
            ryo_core::inbound::use_cases::lint::use_case::LintUseCaseImpl::new(
                ryo_core::application::file_analysis::service::FileAnalysis::new(
                    Arc::clone(&analysis_corpus),
                    Arc::clone(&analysis_language),
                    Arc::clone(&analysis_digest),
                    Arc::clone(&analysis_memory),
                ),
                ryo_core::application::cross_file::service::CrossFile::new(Arc::clone(
                    &analysis_corpus,
                ), std::sync::Arc::new(ryo_core::outbound::publication::factory::execution())),
                Arc::clone(&analysis_corpus),
                spill,
                analysis_language,
                matching,
                query,
                analysis_memory,
                Arc::new(ryo_execution::outbound::operators::factory::use_case(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())))),
std::sync::Arc::new(ryo_execution::outbound::derived::factory::use_case(ryo_execution::outbound::derived::factory::semantics())),
                pool,
                workers,
                Arc::new(row_sink::factory::collect(collector)),
                analysis_digest,
                store,
            )
            .compiling_into(plans)
        },
    })
}

/// Builds the scheduler over the pool of threads runs read files on: `asked` of them, at most as many as the machine has, and as many
/// as it has when none is asked.
pub(crate) fn build_pool(
    workers: scheduler::types::WorkerCount,
) -> Arc<dyn scheduler::interface::SchedulerPort> {
    Arc::new(scheduler::factory::rayon(workers))
}

/// How many workers `build_pool(asked)` runs jobs on.
pub(crate) fn build_workers(
    asked: ryo_values::domain::runs::types::WorkersAsked,
) -> scheduler::types::WorkerCount {
    scheduler::types::WorkerCount(
        ryo_values::domain::runs::service::workers_to_start(
            asked,
            super::connections::machine_workers(),
        )
        .0,
    )
}

/// Chooses the service that checks the repository can be linted with the budget.
pub(crate) fn build_open(budget: ryo_values::domain::runs::types::Budget) -> impl OpenUseCase {
    ryo_core::inbound::use_cases::open::use_case::OpenUseCaseImpl::new(
        Arc::new(language::factory::use_case(Vec::new())),
        Arc::new(memory::factory::sysinfo(budget)),
    )
}

/// Chooses the matching a glob is checked with.
pub(crate) fn build_matching() -> impl MatchingPort {
    matching::factory::globset()
}

/// Which files the run reads, as the corpus adapters take them: the disk walked or as the caller listed, with the files given beside
/// it, or only the files given.
pub(crate) fn build_reading(
    request: &LintRequest,
    inventory: ryo_language::domain::project_context::types::GivenProjectInventory,
) -> Reading {
    let given = |sources: &[super::types::CaseSource]| {
        sources
            .iter()
            .map(|each| CorpusFile {
                executable: each.executable,
                path: FilePath(each.path.0.clone()),
                bytes: FileBytes(each.text.0.as_bytes().to_vec()),
            })
            .collect::<Vec<_>>()
    };
    let beside = given(&request.beside);
    match &request.corpus {
        Corpus::Walked => Reading::Disk(CorpusListing::Walked, beside),
        Corpus::Listed(paths) => Reading::Disk(
            CorpusListing::Listed(paths.iter().map(|path| FilePath(path.0.clone())).collect()),
            beside,
        ),
        Corpus::Given(sources) => {
            let mut files = given(sources);
            files.retain(|file| beside.iter().all(|other| other.path != file.path));
            files.extend(beside);
            Reading::Memory(files, inventory)
        }
    }
}

/// A linked language as the engine's language port takes it: its name, its build and its two use cases. What every edge of the SDK, and
/// the command line, composes the engine with: the use cases are reached through the engine, never through a method of the language.
#[doc(hidden)]
impl From<Language> for language::factory::LanguageConnection {
    fn from(linked: Language) -> Self {
        Self {
            name: ryo_values::domain::descriptions::types::LanguageName(linked.name().0.clone()),
            build: language::types::LanguageBuild(linked.build().0.clone()),
            parse: linked.parse_use_case(),
            project: linked.project_use_case(),
        }
    }
}

/// A pool the caller already runs, as the threads every run reads files on: one pool shared by the caller and the linter.
impl From<Arc<rayon::ThreadPool>> for WorkerPool {
    fn from(pool: Arc<rayon::ThreadPool>) -> Self {
        let workers = scheduler::types::WorkerCount(
            u32::try_from(pool.current_num_threads()).unwrap_or(u32::MAX),
        );
        Self::scheduler(Arc::new(scheduler::factory::rayon_over(pool)), workers)
    }
}

/// Where `path` is, written from the current directory without `.` or `..` components: the same place whatever its spelling.
fn normalized(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let Ok(absolute) = std::path::absolute(path) else {
        return None;
    };
    let mut parts = std::path::PathBuf::new();
    for part in absolute.components() {
        match part {
            std::path::Component::ParentDir => {
                let _popped = parts.pop();
            }
            std::path::Component::CurDir => {}
            kept @ (std::path::Component::Prefix(_)
            | std::path::Component::RootDir
            | std::path::Component::Normal(_)) => parts.push(kept),
        }
    }
    Some(parts)
}

/// The walk's skip for the directory that holds the store: `./` and the path beneath `root`, however either is spelled, when the
/// store is kept beneath the root; none when it is kept elsewhere. A path, not a name: only that directory is skipped.
pub(crate) fn store_skip(
    store: &std::path::Path,
    root: &std::path::Path,
) -> Option<std::path::PathBuf> {
    let (Some(store), Some(root)) = (normalized(store), normalized(root)) else {
        return None;
    };
    let Ok(beneath) = store.strip_prefix(root) else {
        return None;
    };
    if beneath.as_os_str().is_empty() {
        return None;
    }
    // Joined by `/` on every platform, as the walk reads every path it skips.
    let mut skip = std::path::Component::CurDir
        .as_os_str()
        .to_string_lossy()
        .into_owned();
    for part in beneath.components() {
        skip.push(SKIP_SEPARATOR);
        skip.push_str(&part.as_os_str().to_string_lossy());
    }
    Some(std::path::PathBuf::from(skip))
}

/// The dependency content identity using the digest adapter composed by the SDK.
impl From<DependencyBytes<'_>> for DependencyFingerprint {
    fn from(bytes: DependencyBytes<'_>) -> Self {
        use ryo_execution::outbound::digest::interface::DigestPort as _;
        let digest = digest::factory::blake3().digest(
            ryo_execution::outbound::digest::types::DigestBytes(bytes.0),
        );
        Self(digest.0)
    }
}
/// What joins the parts of a path the walk skips, on every platform.
const SKIP_SEPARATOR: char = '/';
