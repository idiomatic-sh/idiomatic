//! The executor the composition roots link: the SDK's `Linter` runs through it. The executor needs this crate's request and report
//! types, so this crate cannot depend on it: the program links it once (`link_executor`) and the `Linter` calls it from then on.

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
pub type Execute = fn(usize, Vec<LintRequest>, &Languages, &Cache, NonZeroUsize) -> Vec<Result<LintReport, LintError>>;

/// The executor a program linked in place of the session's own (the images wrap it to measure): how to run a round, how to close a
/// session, how to retire one.
static LINKED: OnceLock<(Execute, fn(usize), fn(usize))> = OnceLock::new();

/// The executor in use: the one linked, else the session's own.
pub(super) fn in_use() -> (Execute, fn(usize), fn(usize)) {
    LINKED.get().copied().unwrap_or((super::session::execute_round, super::session::release_session, super::session::retire_session))
}

/// The next session of a `Linter`.
pub(super) static NEXT_SESSION: AtomicUsize = AtomicUsize::new(0);

/// Links the executor: once per process, by the program's composition root.
///
/// # Panics
///
/// When an executor is linked already.
#[doc(hidden)]
pub fn link_executor(execute: Execute, close: fn(usize), retire: fn(usize)) {
    assert!(LINKED.set((execute, close, retire)).is_ok(), "one linked executor");
}

/// A report, for the executor: the type is non-exhaustive, so only this crate builds it.
#[doc(hidden)]
#[must_use]
pub fn report(findings: Vec<Finding>, unknowns: Vec<Unknown>, unread: Vec<Unread>) -> LintReport {
    LintReport { dependencies: Vec::new(), findings, unread, unknowns, skipped: Vec::new() }
}

/// A finding, for the executor.
#[doc(hidden)]
#[must_use]
pub fn finding(rule: super::types::RuleId, subject: Slot, key: FindingKey, place: Place, columns: Vec<FindingColumn>) -> Finding {
    Finding { rule, subject, key, place, columns }
}

/// An unknown, for the executor.
#[doc(hidden)]
#[must_use]
pub fn unknown(rule: super::types::RuleId, path: RelativePath, message: Message) -> Unknown {
    Unknown { rule, path, message }
}

/// An unread file, for the executor.
#[doc(hidden)]
#[must_use]
pub fn unread(path: RelativePath, reason: Unreadable) -> Unread {
    Unread { path, reason }
}
