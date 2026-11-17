//! What one worker's open file is done with before it made a node: the node's readers that never run in the file, and its
//! engine uses already done. The making counts them, so the node waits only for its consumers still to come; the node's
//! release forgets them, and so does the file's end, all at once: each count is stamped with its file, and a count of an
//! earlier file is none, so a node the file never made carries nothing into the next file and the end visits no node.
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{EngineUses, NodeReaders, PlanNodeId, RowSource};

/// The readers and engine uses of each node the open file is done with before it made the node, by node id.
#[derive(Debug, Default)]
pub(crate) struct ConsumersDone {
    /// What was done with each node, by node id, and in which file.
    done: Vec<NodeDone>,
    /// The open file, among the files the worker ended.
    file: FileStamp,
}

/// What a file was done with of one node.
#[derive(Clone, Copy, Debug, Default)]
struct NodeDone {
    /// The file the counts are of: of another file than the open one, they are none.
    file: FileStamp,
    /// The readers done.
    readers: NodeReaders,
    /// The engine uses done.
    engine: EngineUses,
}

/// A file among those one worker ended, counted from the first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FileStamp(u32);

impl ConsumersDone {
    /// One reader of `node`, a node of `plan`, is done before the node is made.
    pub(crate) fn reader_done(&mut self, plan: &PhysicalPlan, node: PlanNodeId) {
        if let Some(done) = self.open(plan, node) {
            done.readers.0 = done.readers.0.saturating_add(1);
        }
    }

    /// One engine use of `node`, a node of `plan`, is done before the node is made.
    pub(crate) fn engine_done(&mut self, plan: &PhysicalPlan, node: PlanNodeId) {
        if let Some(done) = self.open(plan, node) {
            done.engine.0 = done.engine.0.saturating_add(1);
        }
    }

    /// What the open file was done with of `node`, a node of `plan`: none yet when its counts are of another file.
    fn open(&mut self, plan: &PhysicalPlan, node: PlanNodeId) -> Option<&mut NodeDone> {
        if self.done.len() < plan.nodes.len() {
            self.done.resize(plan.nodes.len(), NodeDone::default());
        }
        let file = self.file;
        let done = self
            .done
            .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))?;
        if done.file != file {
            *done = NodeDone {
                file,
                ..NodeDone::default()
            };
        }
        Some(done)
    }

    /// What the open file was done with of `node`: none when its counts are of another file.
    fn of(&self, node: PlanNodeId) -> NodeDone {
        self.done
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
            .copied()
            .filter(|done| done.file == self.file)
            .unwrap_or_default()
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
        NodeReaders(readers.0.saturating_sub(self.of(node).readers.0))
    }

    /// The engine uses still to come of `node`, made now, of its plan's `engine`.
    pub(crate) fn engine_left(&self, node: PlanNodeId, engine: EngineUses) -> EngineUses {
        EngineUses(engine.0.saturating_sub(self.of(node).engine.0))
    }

    /// Forgets what was done with `node`: it was released.
    pub(crate) fn forgotten(&mut self, node: PlanNodeId) {
        if let Some(done) = self
            .done
            .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
        {
            *done = NodeDone::default();
        }
    }

    /// The open file ended: what it was done with of every node is forgotten, no node visited. Once in the 2^32 files a
    /// worker ends, the stamps start again from none, each forgotten.
    pub(crate) fn file_ended(&mut self) {
        self.file = FileStamp(self.file.0.wrapping_add(1));
        if self.file == FileStamp::default() {
            self.done.fill(NodeDone::default());
            self.file = FileStamp(1);
        }
    }
}
