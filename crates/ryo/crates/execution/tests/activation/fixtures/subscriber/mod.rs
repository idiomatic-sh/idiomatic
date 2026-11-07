use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Sample {
    /// Rule-relation lookup, relation visit, graph probe.
    pub work: [u64; 3],
    /// Actual visited rule indices in relation and graph lookups.
    pub rule_sums: [u64; 2],
    pub graph_hits: u64,
    pub blocks: u64,
    pub bytes: u64,
}
/// Per-rule accesses to private prepared metadata, counted at their only accessors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Access {
    /// Single-rule metadata reads.
    pub reads: u64,
    /// Sum of the rule indices those reads named.
    pub read_rules: u64,
}
#[derive(Default)]
pub(in crate::fixtures) struct State {
    pub(in crate::fixtures) active: bool,
    pub(in crate::fixtures) starts: usize,
    pub(in crate::fixtures) ends: usize,
    pub(in crate::fixtures) before: (u64, u64),
    pub(in crate::fixtures) sample: Sample,
    pub(in crate::fixtures) access: Access,
}
// Field values borrow an event only while Visit runs; the event name is copied
// to a small ordinal instead of storing an event-owned string.
#[derive(Default)]
pub(in crate::fixtures) struct EventFields {
    event: u8,
    rule: u64,
}
impl tracing::field::Visit for EventFields {
    fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "event" {
            self.event = match value {
                "begin" => 1,
                "end" => 2,
                "relation_rule" => 3,
                "relation_visit" => 4,
                "graph_rule" => 5,
                "graph_hit" => 6,
                "rule_metadata" => 7,
                _ => 0,
            };
        }
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() == "rule" {
            self.rule = value;
        }
    }
}
#[derive(Clone, Default)]
pub(in crate::fixtures) struct Activation(pub(in crate::fixtures) Arc<Mutex<State>>);
impl tracing::Subscriber for Activation {
    fn register_callsite(
        &self,
        _: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        tracing::subscriber::Interest::sometimes()
    }
    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == "ryo::diagnostic::activation"
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = EventFields::default();
        event.record(&mut fields);
        let mut state = self.0.lock().unwrap();
        if fields.event == 1 {
            assert!(!state.active);
            let heap = dhat::HeapStats::get();
            state.before = (heap.total_blocks, heap.total_bytes);
            state.sample = Sample::default();
            state.access = Access::default();
            state.starts += 1;
            state.active = true;
        } else if fields.event == 2 {
            assert!(state.active);
            let heap = dhat::HeapStats::get();
            state.sample.blocks = heap.total_blocks - state.before.0;
            state.sample.bytes = heap.total_bytes - state.before.1;
            state.ends += 1;
            state.active = false;
        } else if state.active && fields.event == 7 {
            state.access.reads += 1;
            state.access.read_rules += fields.rule;
        } else if state.active && fields.event == 6 {
            state.sample.graph_hits += 1;
        } else if state.active && (3..=5).contains(&fields.event) {
            state.sample.work[usize::from(fields.event - 3)] += 1;
            match fields.event {
                3 => state.sample.rule_sums[0] += fields.rule,
                5 => state.sample.rule_sums[1] += fields.rule,
                _ => {}
            }
        }
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}
