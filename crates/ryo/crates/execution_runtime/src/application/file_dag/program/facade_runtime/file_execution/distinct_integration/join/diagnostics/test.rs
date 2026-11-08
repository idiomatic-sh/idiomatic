//! The private owner reports once at normal file shutdown, including failure/empty roots.
use super::super::cases::test::table_of;
use super::super::*;
/// Capture only the opt-in aggregate target, without adding a test dependency.
#[derive(Clone, Default)]
struct JoinDiagnostics {
    /// Recorded fields of each completed invocation, shared with the assertion scope.
    events: std::sync::Arc<std::sync::Mutex<Vec<std::collections::BTreeMap<String, String>>>>,
}

impl tracing::Subscriber for JoinDiagnostics {
    fn register_callsite(
        &self,
        _: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        // Other parallel tests use the no-op dispatcher. Never cache their disabled
        // decision for this thread-local capture, or this capture for their tests.
        tracing::subscriber::Interest::sometimes()
    }
    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == "ryo::diagnostic::stored_join"
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = JoinDiagnosticFields::default();
        event.record(&mut fields);
        self.events
            .lock()
            .expect("diagnostic events")
            .push(fields.0);
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

/// Convert the aggregate event's typed fields into independent expected text.
#[derive(Default)]
struct JoinDiagnosticFields(std::collections::BTreeMap<String, String>);
impl tracing::field::Visit for JoinDiagnosticFields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let _previous = self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let _previous = self.0.insert(field.name().to_owned(), value.to_owned());
    }
}

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
