//! The concrete local implementation executes the complete shared contract, and its event writer keeps its records.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::engine::fixtures::contract::check;
use std::io::Write as _;
use super::writer::{self, DiagnosticEventWriter, RULE_FILE_TARGET, RULE_STEP_TARGET, open, subscriber};
use tracing_subscriber::fmt::writer::MakeWriter as _;

fn concrete_implementation_answers_the_shared_contract() -> Result<(), Box<dyn std::error::Error>> {
    check(&crate::outbound::engine::factory::local(
        ryo::linter::Languages::default(),
        None,
    ));
    Ok(())
}


fn event_writer_flushes_complete_lines_for_live_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("events.jsonl");
    let writer = DiagnosticEventWriter::new(open(&path)?);
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

fn diagnostic_subscriber_writes_readable_jsonl_events() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("events.jsonl");
    let dispatch = tracing::Dispatch::new(subscriber(
        &path,
        ryo_benchmark_contracts::domain::requests::types::DiagnosticOptions {
            execution: ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On,
            steps: ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On,
            ..Default::default()
        },
    )?);
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

fn diagnostic_subscriber_records_step_events_with_nested_context()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("events.jsonl");
    let dispatch = tracing::Dispatch::new(subscriber(
        &path,
        ryo_benchmark_contracts::domain::requests::types::DiagnosticOptions {
            execution: ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On,
            steps: ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On,
            ..Default::default()
        },
    )?);
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

fn ordinary_info_subscriber_does_not_enable_diagnostic_step_events()
-> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish();
    let dispatch = tracing::Dispatch::new(subscriber);
    let enabled = tracing::dispatcher::with_default(
        &dispatch,
        || tracing::enabled!(target: RULE_STEP_TARGET, tracing::Level::TRACE),
    );
    assert!(!enabled);
    let file_program_enabled = tracing::dispatcher::with_default(
        &dispatch,
        || tracing::enabled!(target: writer::FILE_PROGRAM_TARGET, tracing::Level::TRACE),
    );
    assert!(!file_program_enabled);
    Ok(())
}

/// The same subscriber can expose coarse work without enabling it by default.
fn aggregate_profile_is_explicit_and_keeps_counter_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let directory = tempfile::tempdir()?;
    for level in [
        tracing_subscriber::filter::LevelFilter::OFF,
        tracing_subscriber::filter::LevelFilter::TRACE,
    ] {
        let path = directory.path().join(format!("{level}.jsonl"));
        let dispatch = tracing::Dispatch::new(writer::subscriber(
            &path,
            ryo_benchmark_contracts::domain::requests::types::DiagnosticOptions {
                execution: if level == tracing_subscriber::filter::LevelFilter::TRACE {
                    ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On
                } else {
                    ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::Off
                },
                ..Default::default()
            },
        )?);
        tracing::dispatcher::with_default(&dispatch, || {
            tracing::trace!(target: writer::EXECUTION_TARGET, phase = "provider_parse", elapsed_ns = 3_u64);
            tracing::trace!(target: writer::SELECTION_TARGET, event = "file_selections", scanned_rows = 7_u64);
            tracing::trace!(target: writer::WORKING_SET_TARGET, event = "rule_released", retained_bytes = 11_usize);
            tracing::trace!(target: writer::LAYOUT_TARGET, event = "layout_schema", payload_rows = 0_usize);
            tracing::trace!(target: writer::GRAPH_TARGET, event = "graph_work", graph_edges = 13_usize);
            tracing::trace!(target: writer::FILE_PROGRAM_TARGET, event = "file_program", shared_executions = 5_u64);
            tracing::trace!(target: writer::LIFECYCLE_TARGET, phase = "after_files", retained_bytes = 17_usize);
        });
        let output = std::fs::read_to_string(path)?;
        if level == tracing_subscriber::filter::LevelFilter::OFF {
            assert!(output.is_empty());
        } else {
            let events = output
                .lines()
                .map(serde_json::from_str::<serde_json::Value>)
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(events.len(), 7);
            assert!(
                events
                    .iter()
                    .any(|event| event["fields"]["scanned_rows"] == 7)
            );
            assert!(
                events
                    .iter()
                    .any(|event| event["fields"]["retained_bytes"] == 11)
            );
            assert!(
                events
                    .iter()
                    .any(|event| event["fields"]["graph_edges"] == 13)
            );
            assert!(
                events
                    .iter()
                    .any(|event| event["fields"]["shared_executions"] == 5)
            );
        }
    }
    Ok(())
}

fn lifecycle_profile_excludes_per_file_steps_and_other_aggregates()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("lifecycle.jsonl");
    let dispatch = tracing::Dispatch::new(writer::subscriber(
        &path,
        ryo_benchmark_contracts::domain::requests::types::DiagnosticOptions {
            execution: ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On,
            lifecycle_memory: ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On,
            ..Default::default()
        },
    )?);
    tracing::dispatcher::with_default(&dispatch, || {
        tracing::trace!(target: RULE_FILE_TARGET, event = "rule_file_start");
        tracing::trace!(target: RULE_STEP_TARGET, event = "rule_step_start");
        tracing::trace!(target: writer::GRAPH_TARGET, event = "graph_work");
        tracing::trace!(target: writer::EXECUTION_TARGET, phase = "provider_parse");
        tracing::trace!(target: writer::LIFECYCLE_TARGET, phase = "after_files", retained_bytes = 17_usize);
    });
    let text = std::fs::read_to_string(path)?;
    let events = text
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["target"], writer::LIFECYCLE_TARGET);
    assert_eq!(events[0]["fields"]["retained_bytes"], 17);
    Ok(())
}

fn resource_route_profile_preserves_generic_aggregate_payload()
-> Result<(), Box<dyn std::error::Error>> {
    for enabled in [false, true] {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("routes.jsonl");
        let dispatch = tracing::Dispatch::new(writer::subscriber(
            &path,
            ryo_benchmark_contracts::domain::requests::types::DiagnosticOptions {
                resource_routes: if enabled { ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On } else { ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::Off },
                ..Default::default()
            },
        )?);
        tracing::dispatcher::with_default(&dispatch, || {
            tracing::trace!(target: writer::RESOURCE_ROUTES_TARGET, event="resource_routes", file=7, overflow=0, counts="[{\"rule\":2,\"node\":41,\"owner\":false,\"route\":\"scoped_build\",\"family\":\"Equality\",\"calls\":1,\"successes\":1,\"failures\":0}]");
        });
        let text = std::fs::read_to_string(path)?;
        if enabled {
            let event: serde_json::Value = serde_json::from_str(text.trim())?;
            let counts: serde_json::Value = serde_json::from_str(
                event["fields"]["counts"]
                    .as_str()
                    .ok_or("missing route aggregate")?,
            )?;
            assert_eq!(counts[0]["node"], 41);
            assert_eq!(counts[0]["calls"], 1);
            assert_eq!(event["fields"]["overflow"], 0);
        } else {
            assert!(text.is_empty());
        }
    }
    Ok(())
}


type Scenario = fn() -> Result<(), Box<dyn std::error::Error>>;
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("concrete implementation answers the shared contract"),
        word: Word::Succeeds,
        input: || concrete_implementation_answers_the_shared_contract as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("event_writer_flushes_complete_lines_for_live_diagnostics"),
        word: Word::Succeeds,
        input: || event_writer_flushes_complete_lines_for_live_diagnostics as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("diagnostic_subscriber_writes_readable_jsonl_events"),
        word: Word::Succeeds,
        input: || diagnostic_subscriber_writes_readable_jsonl_events as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("diagnostic_subscriber_records_step_events_with_nested_context"),
        word: Word::Succeeds,
        input: || diagnostic_subscriber_records_step_events_with_nested_context as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordinary_info_subscriber_does_not_enable_diagnostic_step_events"),
        word: Word::Succeeds,
        input: || ordinary_info_subscriber_does_not_enable_diagnostic_step_events as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("aggregate_profile_is_explicit_and_keeps_counter_fields"),
        word: Word::Succeeds,
        input: || aggregate_profile_is_explicit_and_keeps_counter_fields as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("lifecycle_profile_excludes_per_file_steps_and_other_aggregates"),
        word: Word::Succeeds,
        input: || lifecycle_profile_excludes_per_file_steps_and_other_aggregates as Scenario,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("resource_route_profile_preserves_generic_aggregate_payload"),
        word: Word::Succeeds,
        input: || resource_route_profile_preserves_generic_aggregate_payload as Scenario,
        expect: Expect::Value(()),
    },
];
#[expect(
    clippy::expect_used,
    reason = "a failed scenario must fail the test"
)]
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario().expect("the scenario holds");
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
