//! `linter lint`: checks, connects, composes, calls the service once.

use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
use ryo_core::inbound::use_cases::lint::types::LintRequest as CoreRequest;
use ryo_core::inbound::use_cases::open::interface::OpenUseCase as _;
use ryo_core::inbound::use_cases::open::types::OpenRequest;

use super::errors::LintError;
use super::schemas;
use super::types::{Cache, LintReport, LintRequest, Linter, Workers};
use crate::domain::capacity::types::{MemoryBudget, WorkerPool};
use crate::domain::language::types::Languages;

/// `linter lint`: checks rules over a repository, with the languages the program linked, on `pool`. The run reads the repository
/// and writes nothing into it: what it leaves for the next run is kept in `cache`, or nowhere.
///
/// # Errors
///
/// `Rule` for a rule whose query is not correct, `MissingParameter` for a parameter nothing is bound to, `TooLarge` when the
/// repository does not fit the memory budget, `Io` when a part of the system fails.
pub fn lint(
    languages: Languages,
    pool: &WorkerPool,
    memory: MemoryBudget,
    cache: Cache,
    request: LintRequest,
) -> Result<LintReport, LintError> {
    lint_with(
        &Linter::new(languages, pool.clone(), memory, cache),
        request,
    )
}

#[expect(
    clippy::multiple_inherent_impl,
    reason = "the constructor is the entry, which the entry file holds; the type keeps only what its sibling entry reads"
)]
impl Linter {
    /// The engine of the languages the program linked, built once over `pool`, the threads the caller built once: every
    /// `lint_with` over it shares its compiler of queries, the plans it compiled, the globs it made and the pool. No pool is
    /// made per run. Every run over it may use at most `memory` and keeps what it leaves for the next in `cache`.
    #[must_use]
    pub fn new(languages: Languages, pool: WorkerPool, memory: MemoryBudget, cache: Cache) -> Self {
        let build = super::connections::build(&languages.0);
        Self::over(super::composition::build_engine(
            languages.0,
            pool,
            build,
            memory,
            cache,
        ))
    }
    /// Select execution explicitly; the same choice controls runtime and persisted identity.
    #[must_use]
    pub fn with_execution(mut self, execution: super::types::ExecutionOptions) -> Self {
        self.set_execution(execution);
        self
    }
}

/// `linter lint` over `linter`: checks rules over a repository, as `lint` does, with no setup of its own.
///
/// # Errors
///
/// As `lint`.
pub fn lint_with(linter: &Linter, request: LintRequest) -> Result<LintReport, LintError> {
    let engine = linter.engine();
    let connections = super::connections::connect(&engine.cache, engine)?;
    let reading = super::composition::build_reading(&request);
    let budget = schemas::budget(engine.memory);
    let mut core: CoreRequest = schemas::request(request)?;
    let opened = super::composition::build_open(budget)
        .open(OpenRequest {
            root: core.session.root.clone(),
            repository: core.session.repository,
        })
        .map_err(schemas::open_error)?
        .session;
    let rows = super::composition::build_rows();
    let settings = super::composition::LinterLintSettings {
        root: opened.root.clone(),
        budget,
    };
    // Opening establishes the root and repository. The caller's declared lookup
    // scope and Given inventory remain the authority for this request.
    let configured = std::mem::replace(&mut core.session, opened);
    core.session.search_scope = configured.search_scope;
    core.session.given_inventory = configured.given_inventory;
    let report = super::composition::build(engine, rows.clone(), reading, &connections, settings)
        .lint(core)
        .map_err(schemas::error)?;
    schemas::report(rows, report)
}

/// The threads every run of the caller reads files on, built once: `workers` of them, at most as many as the machine has. A pool
/// that cannot be started is answered by the first run over it, as `Io` of the scheduler.
impl From<Workers> for WorkerPool {
    fn from(workers: Workers) -> Self {
        let asked = schemas::workers(workers);
        Self::scheduler(
            super::composition::build_pool(asked),
            super::composition::build_workers(asked),
        )
    }
}

/// Fingerprint dependency bytes using the same opaque content identity as lint reports.
#[must_use]
pub fn dependency_fingerprint(bytes: &[u8]) -> super::types::DependencyFingerprint {
    super::composition::dependency_fingerprint(bytes)
}
