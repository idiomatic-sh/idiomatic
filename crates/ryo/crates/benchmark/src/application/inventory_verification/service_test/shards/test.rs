//! Shards verify their rules at once, each rule once, a failure stopping no other rule, and only the caller publishes.
use super::super::super::fixtures::{ObservedInventory, OverlappingExecution};
use super::super::super::service::verify;
use super::super::{ToyExecution, provider, summary_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::inventory::value::InventoryVerdict;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::requests::types::{
    RegisteredCoverage, case_shard::value::ShardCount,
};
use serde_json::json;
use std::borrow::Cow;

/// What a row verifies.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Two shards whose two executable rules wait for each other.
    Concurrent,
    /// Every shard count from one to four over the same three rules.
    EachRuleOnce,
    /// Two shards, the first of which holds a missing rule before a passing one.
    FailureStopsNothing,
    /// Two shards whose results the caller alone publishes.
    OneWriter,
    /// One shard and three over the same registration.
    EqualSummaries,
    /// No shard at all.
    ZeroShards,
}

type Input = Scenario;
type Value = InventoryVerdict;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("shards run concurrently"),
        word: Word::Succeeds,
        input: || Scenario::Concurrent,
        expect: Expect::Value(InventoryVerdict::Failed),
    },
    Case {
        name: CaseName("every rule runs exactly once across shards"),
        word: Word::Succeeds,
        input: || Scenario::EachRuleOnce,
        expect: Expect::Value(InventoryVerdict::Failed),
    },
    Case {
        name: CaseName("a failed rule does not stop its shard or another"),
        word: Word::Succeeds,
        input: || Scenario::FailureStopsNothing,
        expect: Expect::Value(InventoryVerdict::Failed),
    },
    Case {
        name: CaseName("the summary has one writer, in completion order"),
        word: Word::Succeeds,
        input: || Scenario::OneWriter,
        expect: Expect::Value(InventoryVerdict::Failed),
    },
    Case {
        name: CaseName("one shard and three shards publish equal summaries"),
        word: Word::Succeeds,
        input: || Scenario::EqualSummaries,
        expect: Expect::Value(InventoryVerdict::Failed),
    },
    Case {
        name: CaseName("zero shards are refused before any rule runs"),
        word: Word::Refuses,
        input: || Scenario::ZeroShards,
        expect: Expect::Error(Cow::Borrowed("invalid case shard coordinates")),
    },
];

/// Verify the registration of `registration` into `output` over `shards`, through the given execution and store.
fn verify_with(
    registration: &crate::outbound::inventory::types::Registration,
    output: &std::path::Path,
    execution: &dyn crate::outbound::execution::interface::ExecutionPort,
    store: &dyn crate::outbound::inventory::interface::InventoryPort,
    shards: ShardCount,
) -> Result<InventoryVerdict, Error> {
    verify(
        registration,
        output,
        &crate::outbound::suite::factory::local(),
        &crate::outbound::receipts::factory::local(
            crate::outbound::receipts::types::CaseReceipts::Unrecorded,
        ),
        execution,
        store,
        &RegisteredCoverage::Full,
        &crate::outbound::scheduler::factory::threads(),
        shards,
    )
}

fn concurrent() -> Result<InventoryVerdict, Error> {
    let (registration, _context) = provider("concurrent", "bad");
    let output = tempfile::tempdir()?;
    let execution = OverlappingExecution::new(ToyExecution::for_registration(&registration), 2);
    let verdict = verify_with(
        &registration,
        output.path(),
        &execution,
        &crate::outbound::inventory::factory::local(),
        ShardCount(2),
    )?;
    assert_eq!(execution.peak(), 2);
    Ok(verdict)
}

fn each_rule_once() -> Result<InventoryVerdict, Error> {
    let mut verdict = InventoryVerdict::Passed;
    for count in 1..=4 {
        let (registration, _context) = provider("once", "bad");
        let output = tempfile::tempdir()?;
        let execution = ToyExecution::for_registration(&registration);
        verdict = verify_with(
            &registration,
            output.path(),
            &execution,
            &crate::outbound::inventory::factory::local(),
            ShardCount(count),
        )?;
        let mut calls = execution.calls.lock().unwrap().clone();
        calls.sort();
        assert_eq!(calls, ["bad", "good"], "{count} shards");
        let summary = summary_of(output.path());
        assert_eq!(
            summary["counts"],
            json!({"PASS": 1, "FAIL": 2, "DEFERRED": 0}),
            "{count} shards"
        );
        assert_eq!(summary["rules"].as_array().map(Vec::len), Some(3));
    }
    Ok(verdict)
}

fn failure_stops_nothing() -> Result<InventoryVerdict, Error> {
    let (registration, _context) = provider("failing", "bad");
    let output = tempfile::tempdir()?;
    let verdict = verify_with(
        &registration,
        output.path(),
        &ToyExecution::for_registration(&registration),
        &crate::outbound::inventory::factory::local(),
        ShardCount(2),
    )?;
    let summary = summary_of(output.path());
    let statuses = summary["rules"].as_array().map(|rows| {
        rows.iter()
            .map(|row| row["status"].clone())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        statuses,
        Some(vec![json!("FAIL"), json!("FAIL"), json!("PASS")])
    );
    Ok(verdict)
}

fn one_writer() -> Result<InventoryVerdict, Error> {
    let (registration, _context) = provider("writer", "bad");
    let output = tempfile::tempdir()?;
    let store = ObservedInventory::new(crate::outbound::inventory::factory::local());
    let verdict = verify_with(
        &registration,
        output.path(),
        &ToyExecution::for_registration(&registration),
        &store,
        ShardCount(3),
    )?;
    let publications = store.publications();
    assert!(
        publications
            .iter()
            .all(|publication| publication.writers == 1)
    );
    let undecided = &publications[0].reasons;
    let decided = publications
        .iter()
        .map(|publication| {
            publication
                .reasons
                .iter()
                .zip(undecided)
                .filter(|(reason, before)| reason != before)
                .count()
        })
        .collect::<Vec<_>>();
    let running = publications
        .iter()
        .map(|publication| publication.running)
        .collect::<Vec<_>>();
    assert_eq!(decided, [0, 1, 2, 3, 3]);
    assert_eq!(running, [true, true, true, true, false]);
    Ok(verdict)
}

fn equal_summaries() -> Result<InventoryVerdict, Error> {
    let (registration, _context) = provider("equal", "bad");
    let mut summaries = Vec::new();
    let mut verdict = InventoryVerdict::Passed;
    for count in [1, 3] {
        let output = tempfile::tempdir()?;
        verdict = verify_with(
            &registration,
            output.path(),
            &ToyExecution::for_registration(&registration),
            &crate::outbound::inventory::factory::local(),
            ShardCount(count),
        )?;
        summaries.push(summary_of(output.path()));
    }
    assert_eq!(summaries[0], summaries[1]);
    Ok(verdict)
}

fn zero_shards() -> Result<InventoryVerdict, Error> {
    let (registration, _context) = provider("zero", "bad");
    let output = tempfile::tempdir()?;
    let execution = ToyExecution::for_registration(&registration);
    let answer = verify_with(
        &registration,
        output.path(),
        &execution,
        &crate::outbound::inventory::factory::local(),
        ShardCount(0),
    );
    assert!(execution.calls.lock().unwrap().is_empty());
    answer
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Concurrent => concurrent(),
        Scenario::EachRuleOnce => each_rule_once(),
        Scenario::FailureStopsNothing => failure_stops_nothing(),
        Scenario::OneWriter => one_writer(),
        Scenario::EqualSummaries => equal_summaries(),
        Scenario::ZeroShards => zero_shards(),
    }
    .map_err(|error| error.to_string().into())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
