//! `linter lint`: checks, connects, composes, calls the service once.

use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
use ryo_core::outbound::scheduler::interface::SchedulerPort as _;
use ryo_core::inbound::use_cases::lint::types::LintRequest as CoreRequest;
use ryo_core::inbound::use_cases::open::interface::OpenUseCase as _;
use ryo_core::inbound::use_cases::open::types::OpenRequest;
use ryo_core::outbound::language::interface::LanguagePort as _;
use ryo_execution::inbound::use_cases::compile_rules::interface::CompileRulesUseCase as _;
use ryo_execution::inbound::use_cases::compile_rules::types::CompileRequest;

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
}

#[expect(
    clippy::multiple_inherent_impl,
    reason = "the entry file holds the entries over a linter; the constructor is its sibling in the same file"
)]
impl Linter {
    /// Compiles `rules` ahead of any run, into the plans every `lint_with` over this linter shares, so a later run of the same
    /// rules finds them compiled and spends its time on the files. A rule already compiled is not compiled again.
    ///
    /// # Errors
    ///
    /// `Rule` for a rule whose query is not correct, `MissingParameter` for a parameter nothing is bound to, as `lint` answers
    /// them before it reads a file.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::collections::BTreeMap;
    /// use std::num::NonZeroUsize;
    /// use ryo::linter::{Cache, Languages, Linter, MemoryBudget, WorkerPool, Workers};
    ///
    /// let linter = Linter::new(
    ///     Languages::default(),
    ///     WorkerPool::from(Workers(NonZeroUsize::MIN)),
    ///     MemoryBudget::Machine,
    ///     Cache::None,
    /// );
    /// linter.prepare(&BTreeMap::new(), &[]).expect("no rule to compile is compiled");
    /// ```
    pub fn prepare(
        &self,
        rules: &std::collections::BTreeMap<super::types::RuleId, super::types::Rule>,
        hosted: &[super::types::HostRelation],
    ) -> Result<(), LintError> {
        let rules = schemas::core_rules(rules.clone());
        let hosted = schemas::host_relations(hosted.to_vec())?;
        let specs: Vec<_> = rules
            .iter()
            .map(ryo_values::domain::rule_requests::types::RuleSpec::from)
            .collect();
        let engine = self.engine();
        super::composition::build_compilation(engine)
            .compile_rules(CompileRequest {
                rules: &specs,
                hosted: &hosted,
                vocabularies: &engine.language.kind_vocabularies(),
            })
            .map(drop)
            .map_err(|error| LintError::from_core(error.into()))
    }
}

/// `linter lint` over `linter`: checks rules over a repository, as `lint` does, with no setup of its own.
///
/// # Errors
///
/// As `lint`.
pub fn lint_with(linter: &Linter, request: LintRequest) -> Result<LintReport, LintError> {
    if super::executor::LINKED.get().is_some() {
        return linter
            .lint_round(vec![request])
            .pop()
            .unwrap_or_else(|| Err(LintError::Execution { reason: "the executor answered no request".into() }));
    }
    lint_with_engine(linter, request)
}

/// The engine of the build branch over `linter`, which runs when no executor is linked.
fn lint_with_engine(linter: &Linter, mut request: LintRequest) -> Result<LintReport, LintError> {
    schemas::project_context(&mut request)?;
    let engine = linter.engine();
    let connections = super::connections::connect(&engine.cache, engine)?;
    let inventory = request.given_project_inventory.clone().into_engine();
    let reading = super::composition::build_reading(&request, inventory);
    let budget = engine.memory.into_engine();
    let mut core: CoreRequest = schemas::request(request)?;
    let opened = super::composition::build_open(budget)
        .open(OpenRequest {
            root: core.session.root.clone(),
            repository: core.session.repository,
            search_scope: core.session.search_scope.clone(),
            given_inventory: core.session.given_inventory.clone(),
        })
        .map_err(LintError::from_open)?
        .session;
    let rows = super::composition::build_rows();
    let settings = super::composition::LinterLintSettings {
        root: opened.root.clone(),
        budget,
    };
    core.session = opened;
    let report = super::composition::build(engine, rows.clone(), reading, &connections, settings)
        .lint(core)
        .map_err(LintError::from_core)?;
    ryo_operators::spike::dump();
    ryo_operators::spike::memstats_dump();
    schemas::report(rows.take(), report)
}

/// The threads every run of the caller reads files on, built once: `workers` of them, at most as many as the machine has. A pool
/// that cannot be started is answered by the first run over it, as `Io` of the scheduler.
impl From<Workers> for WorkerPool {
    fn from(workers: Workers) -> Self {
        let started = super::composition::build_workers(schemas::workers(workers));
        Self::scheduler(super::composition::build_pool(started), started)
    }
}

impl Linter {
    /// Answers a round of requests over this linter, in request order: a refusal belongs to its request. Every request of a round
    /// shares the session's plans, files and answers. The round runs on the linked executor; with none linked, each request runs on
    /// the engine of the build branch.
    #[must_use]
    pub fn lint_round(&self, requests: Vec<LintRequest>) -> Vec<Result<LintReport, LintError>> {
        if requests.is_empty() {
            return Vec::new();
        }
        let Some(&(execute, _, _)) = super::executor::LINKED.get() else {
            return requests.into_iter().map(|request| lint_with_engine(self, request)).collect();
        };
        let count = requests.len();
        let fail = |reason: String| -> Vec<Result<LintReport, LintError>> {
            (0..count).map(|_| Err(LintError::Io { what: super::types::Port::Scheduler, reason: reason.clone() })).collect()
        };
        let engine = self.engine();
        let Some(workers) = std::num::NonZeroUsize::new(engine.workers.0 as usize) else {
            return fail("the worker pool is empty".into());
        };
        let (mut requests, mut answers) = (Some(requests), Vec::new());
        let mut job = || {
            answers = execute(engine.session, requests.take().unwrap_or_default(), &engine.languages, &engine.cache, engine.memory, workers);
        };
        match Arc::clone(&engine.scheduler).on_owner(&mut job) {
            Ok(()) => answers,
            Err(reason) => fail(reason.to_string()),
        }
    }

    /// The command's last act: publishes what the session must persist, then gives up the session without dropping its plans,
    /// tables and strings one by one. A caller that drops the linter instead gets a clean drop.
    pub fn retire(self) {
        if let Some(&(_, _, retire)) = super::executor::LINKED.get() {
            let session = self.engine().session;
            let _ = Arc::clone(&self.engine().scheduler).on_owner(&mut || retire(session));
        }
        std::mem::forget(self);
    }
}

impl Drop for Linter {
    fn drop(&mut self) {
        if let Some(&(_, close, _)) = super::executor::LINKED.get() {
            let session = self.engine().session;
            let _ = Arc::clone(&self.engine().scheduler).on_owner(&mut || close(session));
        }
    }
}
