//! Diagnostic subscriber records only explicitly selected coarse targets.
use super::{DiagnosticEventWriter, RULE_FILE_TARGET, RULE_STEP_TARGET, subscriber};
use std::io::Write as _;
use tracing_subscriber::fmt::writer::MakeWriter as _;

#[test]
fn event_writer_flushes_complete_lines_for_live_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("events.jsonl");
    let writer = DiagnosticEventWriter::create(&path)?;
    {
        let mut event = writer.make_writer();
        event.write_all(b"{\"event\":\"rule_file_start\"}\n")?;
    }
    assert_eq!(
        std::fs::read_to_string(path)?,
        "{\"event\":\"rule_file_start\"}\n"
    );
    Ok(())
}

#[test]
fn diagnostic_subscriber_writes_readable_jsonl_events() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("events.jsonl");
    let dispatch = tracing::Dispatch::new(subscriber(&path)?);
    tracing::dispatcher::with_default(&dispatch, || {
        tracing::trace!(
            target: RULE_FILE_TARGET,
            event = "rule_file_start",
            rule = "F821",
            file = "example.py",
        );
    });
    let output = std::fs::read_to_string(path)?;
    assert!(output.ends_with('\n'));
    assert!(output.contains("rule_file_start"));
    assert!(output.contains("F821"));
    assert!(output.contains("example.py"));
    Ok(())
}

#[test]
fn diagnostic_subscriber_records_step_events_with_nested_context()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("events.jsonl");
    let dispatch = tracing::Dispatch::new(subscriber(&path)?);
    tracing::dispatcher::with_default(&dispatch, || {
        let span = tracing::trace_span!(
            target: RULE_STEP_TARGET,
            "rule_file",
            rule = "F811",
            file = "example.py",
        );
        let _entered = span.enter();
        let step_span = tracing::trace_span!(
            target: RULE_STEP_TARGET,
            "rule_step",
            invocation_id = 7_u64,
            step_index = 3_usize,
            parent_step_index = 2_usize,
            nested = true,
            operation = "join",
            operation_details = "Join { left: Step(1), right: Relation(Defines) }",
        );
        let _step_entered = step_span.enter();
        tracing::trace!(
            target: RULE_STEP_TARGET,
            event = "rule_step_start",
            invocation_id = 7_u64,
            step_index = 3_u32,
            operation = "join",
        );
    });
    let output = std::fs::read_to_string(path)?;
    assert!(output.ends_with('\n'));
    assert!(output.contains("rule_step_start"));
    assert!(output.contains("F811"));
    assert!(output.contains("example.py"));
    assert!(output.contains("join"));
    assert!(output.contains("invocation_id"));
    assert!(output.contains("parent_step_index"));
    Ok(())
}

#[test]
fn ordinary_info_subscriber_does_not_enable_diagnostic_step_events() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish();
    let dispatch = tracing::Dispatch::new(subscriber);
    let enabled = tracing::dispatcher::with_default(
        &dispatch,
        || tracing::enabled!(target: RULE_STEP_TARGET, tracing::Level::TRACE),
    );
    assert!(!enabled);
}
