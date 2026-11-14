//! One worker's node buffers: the rows of each materialized node of the open file, kept until its last consumer is done, and
//! their capacity kept across files.

use crate::domain::physical_plan::types::{NodeRows, PlanNodeId};

/// The buffers of one worker, reused across files.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
/// use ryo_operators::domain::physical_plan::types::{NodeRows, PlanNodeId};
/// let buffers = FileBuffers::default();
/// assert_eq!(buffers.rows(PlanNodeId(0)), NodeRows::Absent);
/// ```
#[derive(Debug, Default)]
pub struct FileBuffers {
    /// The rows of each node, by node id; a node not run or released holds none.
    nodes: Vec<Option<ryo_values::domain::rows::types::Table>>,
}

impl FileBuffers {
    /// A node's rows, borrowed (cardinality included).
    #[must_use]
    pub fn rows(&self, node: PlanNodeId) -> NodeRows<'_> {
        usize::try_from(node.0).map_or(NodeRows::Absent, |at| {
            self.nodes
                .get(at)
                .and_then(Option::as_ref)
                .map_or(NodeRows::Absent, NodeRows::Materialized)
        })
    }

    /// The nodes that hold rows: the buffer high-water of a run, for the cases that bound it.
    #[cfg(test)]
    pub(crate) fn held(&self) -> impl Iterator<Item = PlanNodeId> + '_ {
        self.nodes
            .iter()
            .zip(0..)
            .filter(|(held, _)| held.is_some())
            .map(|(_, at)| PlanNodeId(at))
    }

    /// The node's last consumer is done: its buffer returns to the worker's pool.
    pub fn release(&mut self, node: PlanNodeId) {
        if let Ok(at) = usize::try_from(node.0)
            && let Some(held) = self.nodes.get_mut(at)
        {
            *held = None;
        }
    }
}
