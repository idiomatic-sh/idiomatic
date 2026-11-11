//! The private owner reports once at normal file shutdown, including failure/empty roots.
use super::super::cases::fixtures::table_of;
use super::super::*;
use super::fixtures::JoinDiagnostics;
use crate::cases::{Case, CaseName, Expect, Word, run};
#[cfg(test)]
use fixtures::execution::*;
#[cfg(test)]
use fixtures::provider::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::plan_parts::types::Side;

/// The left rows of one join invocation against the right rows `(1, 10), (1, 11), (2, 20)`.
#[derive(Clone, Copy, Debug)]
enum Left {
    /// `1, 1, 9`.
    Duplicates,
    /// No rows.
    Empty,
}

/// What the joins and the diagnostic subscriber observed: each invocation's rows or refusal, the
/// aggregates emitted, and of those, how many are a `file_indexes` event of file 0 and how many
/// carry every owner field as a number; and what each invocation's graph owner held once its file finished.
#[derive(Debug, PartialEq)]
struct Observed {
    invocations: [Result<usize, OperatingError>; 4],
    held: [Option<Held>; 4],
    aggregates: usize,
    file_index_events: usize,
    numeric_owner_fields: usize,
}

/// The invocations, each its left rows and the row after which its sink refuses.
type Input = [(Left, Option<usize>); 4];
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("file_index_diagnostic_is_one_aggregate_per_owner_never_per_probe_or_hit"),
    word: Word::Succeeds,
    input: || {
        [
            (Left::Duplicates, None),
            (Left::Empty, None),
            (Left::Duplicates, Some(0)),
            (Left::Duplicates, None),
        ]
    },
    // One normal-shutdown aggregate for each opened owner.
    expect: Expect::Value(Observed {
        invocations: [Ok(4), Ok(0), Err(OperatingError::NotRun), Ok(4)],
        held: [Some(NOTHING_HELD); 4],
        aggregates: 4,
        file_index_events: 4,
        numeric_owner_fields: 4,
    }),
}];

/// Invokes the joins under a subscriber that records the file index diagnostics.
fn subject(invocations: Input) -> Result<Value, Failure> {
    let n = |n| Cell::Number(Integer(n));
    let duplicates = table_of(&[CellKind::Number], &[&[n(1)], &[n(1)], &[n(9)]]);
    let empty = table_of(&[CellKind::Number], &[]);
    let columns = [FactColumn(vec![1, 1, 2]), FactColumn(vec![10, 11, 20])];
    let right = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let observer = JoinDiagnostics::default();
    let captured = observer.events.clone();
    // tracing-core's sole-dispatch fast path consults the registering thread's default, and another
    // libtest thread may have none. A second live registration makes callsite interest consult the
    // registered subscribers instead; only `observer` is installed for event delivery.
    let registration = tracing::Dispatch::new(observer.clone());
    let mut answers = [Ok(0), Ok(0), Ok(0), Ok(0)];
    let mut held = [None; 4];
    tracing::subscriber::with_default(observer, || {
        for ((answer, held), (left, sink_after)) in
            answers.iter_mut().zip(held.iter_mut()).zip(invocations)
        {
            let rows = match left {
                Left::Duplicates => &duplicates,
                Left::Empty => &empty,
            };
            *answer = invoke(
                &op,
                [Rows::Computed(rows), right.clone()],
                &mut Symbols::default(),
                b"",
                true,
                real_port(),
                Faults {
                    sink_after,
                    ..Faults::default()
                },
            )
            .and_then(|actual| {
                *held = Some(actual.held);
                actual.result.map(|table| table.count.0)
            });
        }
    });
    drop(registration);
    let events = captured.lock().map_err(|_| OperatingError::NotRun)?;
    let owner_fields = [
        "entries",
        "eligible_relations",
        "retained_bytes",
        "physical_builds",
        "reuses",
        "refusals",
    ];
    Ok(Observed {
        invocations: answers,
        held,
        aggregates: events.len(),
        file_index_events: events
            .iter()
            .filter(|event| {
                event.get("event").map(String::as_str) == Some("file_indexes")
                    && event.get("file").map(String::as_str) == Some("0")
            })
            .count(),
        numeric_owner_fields: events
            .iter()
            .filter(|event| {
                owner_fields.iter().all(|field| {
                    event
                        .get(*field)
                        .and_then(|value| value.parse::<u64>().ok())
                        .is_some()
                })
            })
            .count(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
