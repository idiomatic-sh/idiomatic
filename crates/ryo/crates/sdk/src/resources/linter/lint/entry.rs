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
    /// of a list shares the session's plans, files and answers. When it returns, everything the cache is to keep is on disk: a host may
    /// end its process without running destructors and lose nothing; dropping the linter frees memory and nothing else depends on it.
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
        let engine = self.engine();
        // A walked corpus is listed here, for the executor takes a closed inventory: a request whose walk fails is answered by its error.
        let mut answers: Vec<Option<Result<LintReport, LintError>>> = Vec::with_capacity(requests.len());
        let mut runnable = Vec::new();
        let mut failed = Vec::new();
        for request in requests {
            match super::walk::listed(request, engine) {
                Ok((listed, unreadable)) => {
                    answers.push(None);
                    runnable.push(listed);
                    failed.push(unreadable);
                }
                Err(error) => answers.push(Some(Err(error))),
            }
        }
        let count = runnable.len();
        let fail = |reason: String| -> Vec<Result<LintReport, LintError>> {
            (0..count).map(|_| Err(LintError::Io { what: super::types::Port::Scheduler, reason: reason.clone() })).collect()
        };
        let ran = if count == 0 {
            Vec::new()
        } else if let Some(workers) = std::num::NonZeroUsize::new(engine.workers.0 as usize) {
            let (mut pending, mut done) = (Some(runnable), Vec::new());
            let mut job = || {
                done = execute(engine.session, pending.take().unwrap_or_default(), &engine.languages, &engine.cache, workers);
            };
            match Arc::clone(&engine.scheduler).on_owner(&mut job) {
                Ok(()) => done,
                Err(reason) => fail(reason.to_string()),
            }
        } else {
            fail("the worker pool is empty".into())
        };
        let mut ran = ran.into_iter().zip(failed);
        answers
            .into_iter()
            .map(|answer| {
                answer.unwrap_or_else(|| match ran.next() {
                    Some((answered, unreadable)) => answered.map(|report| super::walk::naming_unread(report, unreadable)),
                    None => Err(LintError::Execution { reason: "the executor answered fewer requests than it was given".into() }),
                })
            })
            .collect()
    }

    /// Answers `request` as rows: every row its queries emit, none made one by sharing a rule, subject and key, the first `limit` of
    /// them in `findings` and every one counted in `rows_total`. What `ryo query run` answers through.
    ///
    /// # Errors
    ///
    /// As `lint`.
    #[doc(hidden)]
    pub fn lint_rows(&self, request: LintRequest, limit: usize) -> Result<LintReport, LintError> {
        let engine = self.engine();
        let request = super::walk::listed(request, engine)?;
        let Some(workers) = std::num::NonZeroUsize::new(engine.workers.0 as usize) else {
            return Err(LintError::Io { what: super::types::Port::Scheduler, reason: "the worker pool is empty".into() });
        };
        let mut request = Some(request);
        let mut answer = None;
        let mut job = || {
            if let Some(request) = request.take() {
                answer = Some(super::session::execute_rows(engine.session, request, &engine.languages, &engine.cache, workers, limit));
            }
        };
        match Arc::clone(&engine.scheduler).on_owner(&mut job) {
            Ok(()) => answer.unwrap_or_else(|| Err(LintError::Execution { reason: "the executor answered no request".into() })),
            Err(reason) => Err(LintError::Io { what: super::types::Port::Scheduler, reason: reason.to_string() }),
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
