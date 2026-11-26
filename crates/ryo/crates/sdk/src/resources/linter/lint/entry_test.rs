//! The SDK's lint runs on the executor a program linked: a round is answered in request order, one result for each, and a linter's
//! session is closed when it is dropped and retired when it says so.

use super::entry::lint_with;
use super::executor::{link_executor, report};
use super::types::{Cache, Corpus, LintReport, LintRequest, Linter, Workers};
use crate::domain::capacity::types::{MemoryBudget, WorkerPool};
use crate::domain::language::types::{Languages, SourceLanguage};
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The sessions the fake executor was asked to close and to retire, and the rounds it answered.
static CLOSED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
static RETIRED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
static ROUNDS: AtomicUsize = AtomicUsize::new(0);

fn answer(
    _session: usize,
    requests: Vec<LintRequest>,
    _: &Languages,
    _: &Cache,
    _: NonZeroUsize,
) -> Vec<Result<LintReport, super::errors::LintError>> {
    let _ = ROUNDS.fetch_add(1, Ordering::Relaxed);
    requests.iter().map(|_| Ok(report(Vec::new(), Vec::new(), Vec::new()))).collect()
}

fn close(session: usize) {
    CLOSED.lock().expect("closed sessions").push(session);
}

fn retire(session: usize) {
    RETIRED.lock().expect("retired sessions").push(session);
}

fn request() -> LintRequest {
    LintRequest {
        source_language: SourceLanguage::Recognized,
        root: std::path::PathBuf::new(),
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        rules: BTreeMap::new(),
        files: Vec::new(),
        hosted: Vec::new(),
        corpus: Corpus::Walked,
        beside: Vec::new(),
    }
}

fn linter() -> Linter {
    Linter::new(Languages::default(), WorkerPool::from(Workers(NonZeroUsize::MIN)), MemoryBudget::Machine, Cache::None)
}

/// One test holds the process's one linked executor: the three behaviours are read in order on it.
#[test]
fn the_linked_executor_answers_rounds_and_the_session_is_closed_or_retired() {
    link_executor(answer, close, retire);
    let first = linter();
    let rounds = ROUNDS.load(Ordering::Relaxed);
    let answers = first.lint_round(vec![request(), request(), request()]);
    assert_eq!(answers.len(), 3, "one result for each request of the round");
    assert!(answers.iter().all(Result::is_ok));
    assert_eq!(ROUNDS.load(Ordering::Relaxed), rounds + 1, "a round is one call of the executor");
    assert!(lint_with(&first, request()).is_ok(), "a single request is a round of one");
    assert!(first.lint_round(Vec::new()).is_empty(), "no request, no call");
    let second = linter();
    assert_ne!(first.engine().session, second.engine().session, "each linter has its own session");
    let (kept, dropped) = (first.engine().session, second.engine().session);
    first.retire();
    drop(second);
    assert!(RETIRED.lock().expect("retired sessions").contains(&kept), "retire reaches the executor");
    assert!(!CLOSED.lock().expect("closed sessions").contains(&kept), "a retired session is not closed");
    assert!(CLOSED.lock().expect("closed sessions").contains(&dropped), "a dropped linter closes its session");
}
