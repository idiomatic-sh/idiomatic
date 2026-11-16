//! What one worker's open file is done with before it made a node: the node's readers that never run in the file, and its
//! engine uses already done. The making counts them, so the node waits only for its consumers still to come; the node's
//! release forgets them, so a node the file never made carries nothing into the next file.
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{EngineUses, NodeReaders, PlanNodeId, RowSource};

/// The readers and engine uses of each node the open file is done with before it made the node, by node id.
#[derive(Debug, Default)]
pub(crate) struct ConsumersDone {
    /// The readers done, by node id.
    readers: Vec<NodeReaders>,
    /// The engine uses done, by node id.
    engine: Vec<EngineUses>,
}

impl ConsumersDone {
    /// One reader of `node`, a node of `plan`, is done before the node is made.
    pub(crate) fn reader_done(&mut self, plan: &PhysicalPlan, node: PlanNodeId) {
        if self.readers.len() < plan.nodes.len() {
            self.readers.resize(plan.nodes.len(), NodeReaders(0));
        }
        if let Some(done) = self
            .readers
            .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
        {
            done.0 = done.0.saturating_add(1);
        }
    }

    /// One engine use of `node`, a node of `plan`, is done before the node is made.
    pub(crate) fn engine_done(&mut self, plan: &PhysicalPlan, node: PlanNodeId) {
        if self.engine.len() < plan.nodes.len() {
            self.engine.resize(plan.nodes.len(), EngineUses(0));
        }
        if let Some(done) = self
            .engine
            .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
        {
            done.0 = done.0.saturating_add(1);
        }
    }

    /// `node`, of `plan`, does not run in the open file: one reader of each node it reads is done, one per read, as a run of
    /// it gives them back. A node no node reads and no engine use keeps in the plan as rewritten reads nothing its sources
    /// count: a rewrite that replaces a node takes it off its sources' readers (`ordered_existence`'s self join, select and
    /// projection), so retiring its reads again would release a source its running readers still need.
    pub(crate) fn unread(&mut self, plan: &PhysicalPlan, node: PlanNodeId) {
        let Some(physical) = plan
            .nodes
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
        else {
            return;
        };
        if physical.readers == NodeReaders(0) && physical.engine == EngineUses(0) {
            return;
        }
        for source in physical.lowering.sources() {
            if let RowSource::Node(read) = source {
                self.reader_done(plan, read);
            }
        }
    }

    /// The readers still to come of `node`, made now, of its plan's `readers`.
    pub(crate) fn readers_left(&self, node: PlanNodeId, readers: NodeReaders) -> NodeReaders {
        let at = usize::try_from(node.0).unwrap_or(usize::MAX);
        let done = self.readers.get(at).copied().unwrap_or_default();
        NodeReaders(readers.0.saturating_sub(done.0))
    }

    /// The engine uses still to come of `node`, made now, of its plan's `engine`.
    pub(crate) fn engine_left(&self, node: PlanNodeId, engine: EngineUses) -> EngineUses {
        let at = usize::try_from(node.0).unwrap_or(usize::MAX);
        let done = self.engine.get(at).copied().unwrap_or_default();
        EngineUses(engine.0.saturating_sub(done.0))
    }

    /// Forgets what was done with `node`: it was released.
    pub(crate) fn forgotten(&mut self, node: PlanNodeId) {
        let at = usize::try_from(node.0).unwrap_or(usize::MAX);
        if let Some(done) = self.readers.get_mut(at) {
            *done = NodeReaders(0);
        }
        if let Some(done) = self.engine.get_mut(at) {
            *done = EngineUses(0);
        }
    }
}
