// Installed in the benchmark's copy of the released SDK at composition time.
use std::sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
};

use std::num::NonZeroUsize;

use crate::linter::{
    Cache, Finding, FindingColumn, FindingKey, Languages, LintError, LintReport, LintRequest,
    Message, Place, Port, RelativePath, RuleId, Slot, Unknown, Unread, Unreadable, WorkerPool,
};

type Reply = Result<LintReport, LintError>;
type Execute = fn(usize, Vec<LintRequest>, &Languages, &Cache, NonZeroUsize) -> Vec<Reply>;

static ENGINE: OnceLock<(Execute, fn(usize))> = OnceLock::new();
static NEXT_SESSION: AtomicUsize = AtomicUsize::new(0);

pub fn link(execute: Execute, close: fn(usize)) {
    assert!(
        ENGINE.set((execute, close)).is_ok(),
        "one linked benchmark executor"
    );
}

/// A check session using the languages, scheduler, memory budget and cache supplied by its caller.
pub struct Linter {
    languages: Languages,
    pool: WorkerPool,
    cache: Cache,
    session: usize,
}

impl Linter {
    #[must_use]
    pub fn new(languages: Languages, pool: WorkerPool, cache: Cache) -> Self {
        Self {
            languages,
            pool,
            cache,
            session: NEXT_SESSION.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Answer a round in request order. A refusal belongs to its request.
    pub fn lint(&self, requests: Vec<LintRequest>) -> Vec<Result<LintReport, LintError>> {
        if requests.is_empty() {
            return Vec::new();
        }
        let count = requests.len();
        let fail = |reason: String| {
            (0..count)
                .map(|_| {
                    Err(LintError::Io {
                        what: Port::Scheduler,
                        reason: reason.clone(),
                    })
                })
                .collect()
        };
        let Some(&(execute, _)) = ENGINE.get() else {
            return fail("benchmark executor was not linked".into());
        };
        let mut requests = Some(requests);
        let mut answers = Vec::new();
        let mut job = || {
            answers = execute(
                self.session,
                requests.take().expect("one scheduled job"),
                &self.languages,
                &self.cache,
                NonZeroUsize::new(self.pool.workers().0 as usize).expect("worker pool is nonempty"),
            );
        };
        let scheduled = self.pool.clone().of().on_owner(&mut job);
        match scheduled {
            Ok(()) => answers,
            Err(reason) => fail(reason.to_string()),
        }
    }
}

impl std::fmt::Debug for Linter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Linter").finish_non_exhaustive()
    }
}

impl Drop for Linter {
    fn drop(&mut self) {
        if let Some(&(_, close)) = ENGINE.get() {
            let _ = self.pool.clone().of().on_owner(&mut || close(self.session));
        }
    }
}

// These constructors are private to the linked benchmark composition, outside
// the lint resource. The SDK's non-exhaustive types retain their original shape.
pub fn report(findings: Vec<Finding>, unknowns: Vec<Unknown>, unread: Vec<Unread>) -> LintReport {
    LintReport {
        dependencies: Vec::new(),
        findings,
        unread,
        unknowns,
        skipped: Vec::new(),
    }
}

pub fn finding(
    rule: RuleId,
    subject: Slot,
    key: FindingKey,
    place: Place,
    columns: Vec<FindingColumn>,
) -> Finding {
    Finding {
        rule,
        subject,
        key,
        place,
        columns,
    }
}

pub fn unknown(rule: RuleId, path: RelativePath, message: Message) -> Unknown {
    Unknown {
        rule,
        path,
        message,
    }
}

pub fn unread(path: RelativePath, reason: Unreadable) -> Unread {
    Unread { path, reason }
}
