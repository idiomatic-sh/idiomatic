//! Phase observer isolation and resource boundary proofs.

use std::thread::ThreadId;
use std::time::Duration;

use tracing_subscriber::layer::SubscriberExt as _;

use super::{Log, Phase, Phases};

/// A wall time of `ticks` milliseconds.
fn time(ticks: u64) -> Duration {
    Duration::from_millis(ticks)
}

/// The id of the thread that runs the test, and the id of another thread.
fn threads() -> (ThreadId, ThreadId) {
    let other = std::thread::spawn(|| std::thread::current().id())
        .join()
        .expect("a thread that ends");
    (std::thread::current().id(), other)
}

/// A phase entered again inside itself is one phase, from its first entry to its last exit.
#[test]
fn phases_succeeds_with_exact_outer_pairs_and_ignores_nested_reentry() {
    let (own, _) = threads();
    let mut log = Log::default();
    log.create(1, "run", own);
    log.enter(1, own, time(10));
    log.enter(1, own, time(20));
    log.exit(1, own, time(30));
    log.exit(1, own, time(40));
    assert_eq!(
        log.done,
        vec![Phase {
            name: "run".to_owned(),
            entered: time(10),
            left: time(40)
        }]
    );
}

/// An entry on another thread than the one that created the span is no part of the phase.
#[test]
fn phases_succeeds_without_worker_entries_or_foreign_creator_spans() {
    let (own, worker) = threads();
    let mut log = Log::default();
    log.create(1, "run", own);
    log.enter(1, worker, time(5));
    log.exit(1, worker, time(6));
    assert!(log.done.is_empty());
    log.enter(1, own, time(10));
    log.exit(1, own, time(30));
    assert_eq!(log.done.len(), 1);
    assert_eq!(log.done[0].entered, time(10));
}

/// A span that has a span above it is not an outer phase, and the phases are in the order they ended with each entered before it was left.
#[test]
fn phases_succeeds_with_monotonic_process_cpu_and_clock_brackets() {
    let phases = Phases::new();
    let subscriber = tracing_subscriber::registry().with(phases.clone());
    tracing::subscriber::with_default(subscriber, || {
        tracing::info_span!("parse").in_scope(|| {
            tracing::info_span!("inner").in_scope(|| {});
        });
        tracing::info_span!("run").in_scope(|| {});
    });
    let log = phases.log.lock().expect("the log");
    let names: Vec<&str> = log.done.iter().map(|phase| phase.name.as_str()).collect();
    assert_eq!(names, vec!["parse", "run"]);
    assert!(log.done.iter().all(|phase| phase.entered <= phase.left));
}

/// An exit with no entry, an exit of a span nobody created, and a phase never left are no phase.
#[test]
fn phases_refuses_incomplete_and_out_of_order_diagnostic_sequences() {
    let (own, _) = threads();
    let mut log = Log::default();
    log.exit(7, own, time(1));
    log.create(1, "run", own);
    log.exit(1, own, time(2));
    log.enter(1, own, time(3));
    assert!(log.done.is_empty());
    log.close(1);
    log.exit(1, own, time(4));
    assert!(log.done.is_empty());
}

/// A phase a panic unwinds through is still reported, and the caller's panic is the one it sees.
#[test]
fn phases_succeeds_with_unwinding_without_changing_the_caller_refusal() {
    let phases = Phases::new();
    let subscriber = tracing_subscriber::registry().with(phases.clone());
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            tracing::info_span!("run").in_scope(|| panic!("the caller refuses"));
        });
    }));
    let message = caught
        .expect_err("the panic goes on")
        .downcast::<&str>()
        .expect("the caller's message");
    assert_eq!(*message, "the caller refuses");
    let log = phases.log.lock().expect("the log");
    assert_eq!(log.done.len(), 1);
}

/// Observation off the caller's scope sees nothing: the caller's own dispatch is back once the scope ends.
#[test]
fn phases_succeeds_with_off_preserving_the_callers_dispatch_and_default() {
    let phases = Phases::new();
    let subscriber = tracing_subscriber::registry().with(phases.clone());
    let answer = tracing::subscriber::with_default(subscriber, || {
        tracing::info_span!("run").in_scope(|| 7_i32)
    });
    tracing::info_span!("after").in_scope(|| {});
    let log = phases.log.lock().expect("the log");
    assert_eq!(answer, 7_i32);
    assert_eq!(log.done.len(), 1);
    assert_eq!(log.done[0].name, "run");
}
