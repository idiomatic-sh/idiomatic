//! Deterministic wall attribution contracts; no timing thresholds or sleeps.

use super::{Clock, SpanClock, State};
use std::time::Duration;

fn time(ticks: u64) -> Duration {
    Duration::from_millis(ticks)
}

fn state() -> State {
    let mut state = State::default();
    for (id, parent, name) in [
        (1, None, "parent"),
        (2, Some(1), "first"),
        (3, Some(1), "second"),
    ] {
        state.spans.extend([(
            id,
            SpanClock {
                clock: Clock::default(),
                parent,
                key: ("crate".into(), "hex".into(), name.into()),
            },
        )]);
    }
    state
}

#[test]
fn distinct_overlapping_children_are_subtracted_once() {
    let mut state = state();
    for (id, entered, tick) in [
        (1, true, 0),
        (2, true, 10),
        (3, true, 30),
        (2, false, 50),
        (3, false, 70),
        (1, false, 100),
    ] {
        state.transition(id, entered, time(tick));
    }
    let actual = state
        .spans
        .get(&1)
        .map(|span| (span.clock.busy, span.clock.own));
    assert_eq!(actual, Some((time(100), time(40))));
}

#[test]
fn child_activity_only_subtracts_when_the_parent_is_active() {
    let mut state = state();
    for (id, entered, tick) in [
        (2, true, 0),
        (1, true, 20),
        (2, false, 40),
        (1, false, 60),
        (3, true, 70),
        (1, true, 80),
        (3, false, 90),
        (1, false, 100),
    ] {
        state.transition(id, entered, time(tick));
    }
    let actual = state
        .spans
        .get(&1)
        .map(|span| (span.clock.busy, span.clock.own));
    assert_eq!(actual, Some((time(60), time(30))));
}

#[test]
fn overlapping_parent_reentry_counts_wall_once() {
    let mut clock = Clock::default();
    assert!(clock.enter(time(0)));
    assert!(!clock.enter(time(20)));
    assert!(!clock.exit(time(40)));
    assert!(clock.exit(time(60)));
    assert!(clock.enter(time(80)));
    assert!(clock.exit(time(100)));
    assert_eq!((clock.busy, clock.own), (time(80), time(80)));
}

#[test]
fn a_child_reentered_on_another_thread_stays_active_until_its_last_exit() {
    let mut state = state();
    for (id, entered, tick) in [
        (1, true, 0),
        (2, true, 10),
        (2, true, 20),
        (2, false, 40),
        (2, false, 70),
        (1, false, 100),
    ] {
        state.transition(id, entered, time(tick));
    }
    assert_eq!(
        state.spans.get(&1).map(|span| span.clock.own),
        Some(time(40))
    );
    assert_eq!(
        state.spans.get(&2).map(|span| span.clock.busy),
        Some(time(60))
    );
}

#[test]
fn the_subscriber_reports_closed_port_spans_and_labels_wall_attribution() {
    use tracing_subscriber::layer::SubscriberExt as _;
    // `Off` keeps nothing alive: its guard is dropped at once, as a command that asks for no timings drops it.
    drop(super::install(
        super::Timings::Off,
        std::time::Instant::now(),
    ));
    // A local dispatch exercises subscriber behavior without changing the process global.
    let tree = super::TimingsTree::default();
    tracing::subscriber::with_default(tracing_subscriber::registry().with(tree.clone()), || {
        let parent = tracing::info_span!(target: "owner::port", "parent");
        parent.in_scope(|| {
            let child = tracing::info_span!(target: "owner::port", "child");
            child.in_scope(|| {});
        });
    });
    let mut output = Vec::new();
    tree.print(time(100), &mut output).unwrap();
    let output = String::from_utf8_lossy(&output);
    assert!(output.contains("WALL attribution; command-entry region"));
    assert!(output.contains("not whole-process CPU"));
    assert!(output.contains("    parent "));
    assert!(output.contains("    child "));
}

#[test]
fn capture_events_work_without_timings_and_only_for_explicit_targets() {
    use tracing_subscriber::layer::SubscriberExt as _;
    let output = std::sync::Arc::new(tempfile::tempfile().expect("diagnostic output"));
    let subscriber = tracing_subscriber::registry().with(super::events(
        &["ryo::capture"],
        std::sync::Arc::clone(&output),
    ));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "ryo::capture", rows = 42_u32, "recorded");
        tracing::info!(target: "another::port", "excluded");
        tracing::debug!(target: "ryo::capture", "excluded_debug");
    });
    let text = captured_text(&output);
    assert!(text.contains("recorded"));
    assert!(text.contains("rows=42"));
    assert!(!text.contains("excluded"));
}

#[test]
fn capture_events_and_port_timing_are_both_observed() {
    use tracing_subscriber::layer::SubscriberExt as _;
    let output = std::sync::Arc::new(tempfile::tempfile().expect("diagnostic output"));
    let tree = super::TimingsTree::default();
    let subscriber = tracing_subscriber::registry()
        .with(tree.clone())
        .with(super::events(
            &["ryo::capture"],
            std::sync::Arc::clone(&output),
        ));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info_span!(target: "owner::port", "evaluate").in_scope(|| {
            tracing::info!(target: "ryo::capture", "captured_inside_port");
        });
    });
    assert!(captured_text(&output).contains("captured_inside_port"));
    let mut report = Vec::new();
    tree.print(time(100), &mut report).expect("report");
    assert!(String::from_utf8_lossy(&report).contains("    evaluate "));
}

fn captured_text(file: &std::fs::File) -> String {
    use std::io::Seek as _;
    let mut reader = file;
    reader.rewind().expect("rewind");
    std::io::read_to_string(reader).expect("read diagnostics")
}

#[test]
fn a_second_run_in_one_process_installs_its_subscriber_for_itself() {
    // The first run may make its subscriber the process's; a second, in the same process, gets one of its own for the run
    // instead of an error: a command never fails because of tracing.
    let first = super::install(super::Timings::Off, std::time::Instant::now());
    let second = super::install(super::Timings::Off, std::time::Instant::now());
    let scoped = format!("{second:?}").contains("Some(");
    drop(second);
    drop(first);
    assert!(scoped, "the second run has a subscriber of its own");
}
