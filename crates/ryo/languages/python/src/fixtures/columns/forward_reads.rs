//! Reads preceding a local declaration in the evaluation schedule.
use super::ColumnFile;
use crate::domain::model::flow::types::{FlowEventKind, FlowState};
use crate::domain::model::view::types::{ModelPart, ModelView};
use crate::domain::node::service::table_id;
use std::collections::HashMap;

impl ColumnFile<'_, '_> {
    /// A read starts before its name's first store or bare annotation in the same namespace and execution phase.
    pub fn visit_pre_declaration_reads(&mut self, mut visit: impl FnMut(u32, u32, u32, u64)) -> bool {
        let read = self.file.semantics(ModelPart::Flow);
        let flow = &read.model.flow;
        if flow.state == FlowState::Incomplete {
            return false;
        }
        let mut first = HashMap::new();
        for event in &flow.events {
            if matches!(event.kind, FlowEventKind::Bind | FlowEventKind::Annotation) {
                first.entry((event.scope, event.phase, event.name)).or_insert(event.ordinal.0);
            }
        }
        for event in &flow.events {
            if event.kind == FlowEventKind::Read
                && first.get(&(event.scope, event.phase, event.name)).is_some_and(|&at| event.ordinal.0 < at)
            {
                let id = |node| table_id(read.tree.segments, node).0;
                visit(id(event.at), id(event.scope), id(event.phase), event.ordinal.0 as u64);
            }
        }
        true
    }
}
