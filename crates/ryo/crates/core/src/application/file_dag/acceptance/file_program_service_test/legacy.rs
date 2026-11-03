//! Explicit legacy retention oracle for existing acceptance comparisons.
//! Production file execution uses FileDagExecution and never constructs this state.
use crate::domain::file_program::types::{FileProgram, RequestGraph};
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

pub(super) fn program(
    graph: Arc<RequestGraph>,
    active: impl Fn(usize) -> bool,
    complete: &[Relation],
) -> FileProgram {
    tracing::trace!(target: "ryo::diagnostic::file_program", event = "complete_relations",
            complete = ?complete);
    let mut remaining = vec![0_usize; graph.nodes.len()];
    for (rule, consumers) in graph.consumers.iter().enumerate() {
        if !active(rule) {
            continue;
        }
        for node in consumers {
            if !graph.nodes[node.0]
                .relations
                .iter()
                .all(|relation| complete.contains(relation))
            {
                continue;
            }
            remaining[node.0] += 1;
        }
    }
    FileProgram {
        graph,
        remaining,
        entries: Vec::new(),
        shared_executions: 0,
        peak_retained_cells: 0,
    }
}
