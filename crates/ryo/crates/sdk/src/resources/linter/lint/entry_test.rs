//! The SDK's lint runs on the executor a test linked: a list is answered in request order, one result for
//! each, and a linter's session is closed when it is dropped.

use super::executor::link_executor;
use super::executor::report;
use super::types::{Cache, Corpus, LintReport, LintRequest, Linter, Workers};
use crate::domain::capacity::types::WorkerPool;
use crate::domain::language::types::{Languages, SourceLanguage};
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The sessions the fake executor was asked to close and to retire, and the rounds it answered.
static CLOSED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
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

/// A directory with nothing in it, which a walked request reads: this test is about the executor's linking, not the walk.
fn empty_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("ryo-sdk-entry-test-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("an empty directory");
    root
}

fn request() -> LintRequest {
    LintRequest {
        source_language: SourceLanguage::Recognized,
        root: empty_root(),
        project_search_scope: Default::default(),
        rules: BTreeMap::new(),
        files: Vec::new(),
        hosted: Vec::new(),
        corpus: Corpus::Walked,
        beside: Vec::new(),
    }
}

fn linter() -> Linter {
    Linter::new(Languages::default(), WorkerPool::from(Workers(NonZeroUsize::MIN)), Cache::None)
}

/// One test holds the process's one linked executor: the three behaviours are read in order on it.
#[test]
fn the_linked_executor_answers_lists_and_a_dropped_linter_closes_its_session() {
    link_executor(answer, close);
    let first = linter();
    let rounds = ROUNDS.load(Ordering::Relaxed);
    let answers = first.lint(vec![request(), request(), request()]);
    assert_eq!(answers.len(), 3, "one result for each request of the round");
    assert!(answers.iter().all(Result::is_ok));
    assert_eq!(ROUNDS.load(Ordering::Relaxed), rounds + 1, "a round is one call of the executor");
    assert!(first.lint(vec![request()]).pop().is_some_and(|answer| answer.is_ok()), "a single request is a list of one");
    assert!(first.lint(Vec::new()).is_empty(), "no request, no call");
    let second = linter();
    assert_ne!(first.engine().session, second.engine().session, "each linter has its own session");
    let dropped = second.engine().session;
    drop(second);
    assert!(CLOSED.lock().expect("closed sessions").contains(&dropped), "a dropped linter closes its session");
    let kept = first.engine().session;
    drop(first);
    assert!(CLOSED.lock().expect("closed sessions").contains(&kept), "every linter closes its session when it is dropped");
}
