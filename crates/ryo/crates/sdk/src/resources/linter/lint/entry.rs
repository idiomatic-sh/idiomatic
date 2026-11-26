//! `linter lint`: one linter over the languages a program linked, which lints lists of requests; dropping it closes its session.

use std::sync::Arc;

use super::errors::LintError;
use super::types::{Cache, LintReport, LintRequest, Linter};
use crate::domain::capacity::types::WorkerPool;
use crate::domain::language::types::Languages;

#[expect(
    clippy::multiple_inherent_impl,
    reason = "the constructor is the entry, which the entry file holds; the type keeps only what its sibling entry reads"
)]
impl Linter {
    /// The linter over the languages the program linked, built once over `pool`, the threads the caller built once: every `lint`
    /// over it shares its session (the plans, the files read, the answers kept in `cache`). The engine bounds its own memory.
    #[must_use]
    pub fn new(languages: Languages, pool: WorkerPool, cache: Cache) -> Self {
        Self::over(super::composition::build_engine(languages.0, pool, cache))
    }

    /// Answers a list of requests, in request order: a refusal belongs to its request. A list of one is a single call. Every request
    /// of a list shares the session's plans, files and answers.
    ///
    /// # Errors
    ///
    /// One result for each request: `Rule` for a rule whose query is not correct, `MissingParameter` for a parameter nothing is
    /// bound to, `Io` when a part of the system fails.
    #[must_use]
    pub fn lint(&self, requests: Vec<LintRequest>) -> Vec<Result<LintReport, LintError>> {
        if requests.is_empty() {
            return Vec::new();
        }
        let (execute, _) = super::executor::in_use();
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
            answers = execute(engine.session, requests.take().unwrap_or_default(), &engine.languages, &engine.cache, workers);
        };
        match Arc::clone(&engine.scheduler).on_owner(&mut job) {
            Ok(()) => answers,
            Err(reason) => fail(reason.to_string()),
        }
    }
}

impl Drop for Linter {
    fn drop(&mut self) {
        let (_, close) = super::executor::in_use();
        let session = self.engine().session;
        drop(Arc::clone(&self.engine().scheduler).on_owner(&mut || close(session)));
    }
}

/// The threads every run of the caller reads files on, built once: `workers` of them, at most as many as the machine has. A pool
/// that cannot be started is answered by the first run over it, as `Io` of the scheduler.
impl From<super::types::Workers> for WorkerPool {
    fn from(workers: super::types::Workers) -> Self {
        let started = super::composition::build_workers(super::schemas::workers(workers));
        Self::scheduler(super::composition::build_pool(started), started)
    }
}
