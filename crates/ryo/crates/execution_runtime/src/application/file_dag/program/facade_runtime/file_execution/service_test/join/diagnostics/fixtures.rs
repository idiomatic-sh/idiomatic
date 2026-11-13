//! The support of the `diagnostics` cases.

/// Capture only the opt-in aggregate target, without adding a test dependency.
#[derive(Clone, Default)]
pub(super) struct JoinDiagnostics {
    /// Recorded fields of each completed invocation, shared with the assertion scope.
    pub(super) events:
        std::sync::Arc<std::sync::Mutex<Vec<std::collections::BTreeMap<String, String>>>>,
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
pub(super) struct JoinDiagnosticFields(pub(super) std::collections::BTreeMap<String, String>);

impl tracing::field::Visit for JoinDiagnosticFields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let _previous = self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let _previous = self.0.insert(field.name().to_owned(), value.to_owned());
    }
}
