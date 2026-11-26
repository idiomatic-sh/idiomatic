//! The executor a linter runs on: the session's own, or, for the images that measure, a wrapper of it that the benchmark package links.

use super::errors::LintError;
use super::types::{Cache, Finding, FindingColumn, FindingKey, LintReport, LintRequest, Message, Unknown};
use crate::domain::cell::types::Slot;
use crate::domain::file::types::{RelativePath, Unread, Unreadable};
use crate::domain::language::types::Languages;
use crate::domain::place::types::Place;
use std::num::NonZeroUsize;
use std::sync::OnceLock;
use std::sync::atomic::AtomicUsize;

/// Answers a round of requests in request order, one result for each: a refusal belongs to its request.
pub(crate) type Execute = fn(usize, Vec<LintRequest>, &Languages, &Cache, NonZeroUsize) -> Vec<Result<LintReport, LintError>>;

/// The executor an image linked in place of the session's own: how to run a round and how to close a session.
static LINKED: OnceLock<(Execute, fn(usize))> = OnceLock::new();

/// The next session of a `Linter`.
pub(super) static NEXT_SESSION: AtomicUsize = AtomicUsize::new(0);

/// The executor in use: the one an image linked, else the session's own.
pub(super) fn in_use() -> (Execute, fn(usize)) {
    LINKED.get().copied().unwrap_or((super::session::execute_round, super::session::release_session))
}

/// Links an executor in place of the session's own: once per process, by the benchmark package's main, which wraps the session's to
/// measure it. Only with the `bench-hooks` feature, which a product binary never switches on.
///
/// # Panics
///
/// When an executor is linked already.
#[cfg(feature = "bench-hooks")]
pub fn link_executor(execute: Execute, close: fn(usize)) {
    assert!(LINKED.set((execute, close)).is_ok(), "one linked executor");
}

/// A report, for the session: the type is non-exhaustive, so only this crate builds it.
#[must_use]
pub(super) fn report(findings: Vec<Finding>, unknowns: Vec<Unknown>, unread: Vec<Unread>) -> LintReport {
    LintReport { dependencies: Vec::new(), findings, unread, unknowns, skipped: Vec::new(), rows_total: 0 }
}

/// A finding, for the session.
#[must_use]
pub(super) fn finding(rule: super::types::RuleId, subject: Slot, key: FindingKey, place: Place, columns: Vec<FindingColumn>) -> Finding {
    Finding { rule, subject, key, place, columns }
}

/// An unknown, for the session.
#[must_use]
pub(super) fn unknown(rule: super::types::RuleId, path: RelativePath, message: Message) -> Unknown {
    Unknown { rule, path, message }
}

/// An unread file, for the session.
#[must_use]
pub(super) fn unread(path: RelativePath, reason: Unreadable) -> Unread {
    Unread { path, reason }
}
