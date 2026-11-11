//! The private owner reports once at normal file shutdown, including failure/empty roots.
use super::super::super::super::*;
use super::super::cases::fixtures::table_of;
use super::super::*;
use super::fixtures::JoinDiagnostics;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::{plan_parts::types::Side, rows::types::RowCount};

#[test]
fn file_index_diagnostic_is_one_aggregate_per_owner_never_per_probe_or_hit() {
    let n = |n| Cell::Number(Integer(n));
    let left = table_of(&[CellKind::Number], &[&[n(1)], &[n(1)], &[n(9)]]);
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
    tracing::subscriber::with_default(observer, || {
        for (rows, sink_after, expected) in [
            (&left, None, 4),
            (&empty, None, 0),
            (&left, Some(0), 0),
            (&left, None, 4),
        ] {
            let actual = invoke(
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
            .unwrap();
            if sink_after.is_some() {
                assert!(matches!(actual.result, Err(OperatingError::NotRun)));
            } else {
                assert_eq!(actual.result.unwrap().count, RowCount(expected));
            }
        }
    });
    drop(registration);
    let events = captured.lock().unwrap();
    assert_eq!(
        events.len(),
        4,
        "one normal-shutdown aggregate for each opened owner: {events:?}"
    );
    for event in events.iter() {
        assert_eq!(event.get("event").map(String::as_str), Some("file_indexes"));
        assert_eq!(event.get("file").map(String::as_str), Some("0"));
        for field in [
            "entries",
            "eligible_relations",
            "retained_bytes",
            "physical_builds",
            "reuses",
            "refusals",
        ] {
            assert!(
                event
                    .get(field)
                    .is_some_and(|value| value.parse::<u64>().is_ok()),
                "owner field {field}: {event:?}"
            );
        }
    }
}
