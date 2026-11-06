//! Deterministic wall attribution contracts; no timing thresholds or sleeps.

use super::{Clock, SpanClock, State};
use std::time::Duration;

fn time(ticks: u64) -> Duration {
    Duration::from_millis(ticks)
}

fn state() -> State {
    State {
        spans: [
            (1, None, "parent"),
            (2, Some(1), "first"),
            (3, Some(1), "second"),
        ]
        .into_iter()
        .map(|(id, parent, name)| {
            (
                id,
                SpanClock {
                    clock: Clock::default(),
                    parent,
                    key: ("crate".into(), "hex".into(), name.into()),
                },
            )
        })
        .collect(),
        ..State::default()
    }
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
    drop(super::install(super::Timings::Off, std::time::Instant::now()).unwrap());
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
    tree.print(time(100), &mut output);
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
    tree.print(time(100), &mut report);
    assert!(String::from_utf8_lossy(&report).contains("    evaluate "));
}

/// A query run under the tree attributes its wall time to ryo's own stages, not only to a language's library.
#[test]
fn a_query_run_reports_rows_of_ryo_itself() {
    use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
    use ryo_core::inbound::use_cases::lint::types::{LintRequest, Rule};
    use ryo_values::domain::runs::types::AnswerShape;
    use ryo_values::domain::runs::types::RuleId;
    use tracing_subscriber::layer::SubscriberExt as _;
    let tree = super::TimingsTree::default();
    let subscriber = tracing_subscriber::registry().with(tree.clone());
    let request = LintRequest {
        source_language: None,
        session: ryo_values::domain::runs::types::Session {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
            root: ryo_language::inbound::use_cases::project::types::RootPath(
                "tests/fixtures/sample".into(),
            ),
        },
        files: Vec::new(),
        rules: vec![Rule {
            observations: Vec::new(),
            id: RuleId("1".into()),
            query: ryo_values::domain::query_tokens::types::QueryBundle::from(
                ryo_values::domain::plan_parts::types::QueryText(
                    "let f = select * from \"function\"; emit f subject file key(span);".into(),
                ),
            ),
            applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
            parameters: Vec::new(),
            include: Vec::new(),
            exclude: Vec::new(),
        }],
        hosted: Vec::new(),
        answer: AnswerShape::Rows(ryo_values::domain::row_texts::types::RowTotal(20)),
    };
    tracing::subscriber::with_default(subscriber, || {
        let _report = crate::resources::query::run::composition::build(
            Box::new(std::io::sink()),
            &crate::resources::edge_at(
                ryo_language::inbound::use_cases::project::types::RootPath(".".into()),
                ryo_language::inbound::use_cases::project::types::RootPath(".".into()),
                crate::output::Format::Terminal,
            ),
            ryo_core::outbound::lint_store::types::Store::default(),
            ryo_language::inbound::use_cases::project::types::RootPath(
                "tests/fixtures/sample".into(),
            ),
            ryo_values::domain::runs::types::Budget::Machine,
            ryo_values::domain::runs::types::WorkersAsked(0),
        )
        .lint(request)
        .expect("the query runs");
    });
    let mut report = Vec::new();
    tree.print(time(100), &mut report);
    let report = String::from_utf8_lossy(&report);
    for row in ["ryo_core", "    read ", "    compile "] {
        assert!(report.contains(row), "no `{row}` row in:\n{report}");
    }
}

/// The Chrome trace is written in the temporary directory, never in the directory the command runs in, and its path is said.
#[test]
fn the_chrome_trace_is_written_outside_the_working_directory_and_said() {
    let mut said = Vec::new();
    let file = super::trace_file(&mut said);
    assert!(file.starts_with(std::env::temp_dir()), "{}", file.display());
    let here = std::env::current_dir().expect("a working directory");
    assert!(!file.starts_with(&here) || std::env::temp_dir().starts_with(&here));
    assert!(String::from_utf8_lossy(&said).contains(&*file.to_string_lossy()));
}

fn captured_text(file: &std::fs::File) -> String {
    use std::io::Seek as _;
    let mut reader = file;
    reader.rewind().expect("rewind");
    std::io::read_to_string(reader).expect("read diagnostics")
}
