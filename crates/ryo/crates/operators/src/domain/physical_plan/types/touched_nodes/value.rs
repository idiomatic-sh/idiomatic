//! The nodes one worker's open file touched, each once: those it gave rows or a feed. The file's end releases them and no
//! other node, so its work is the file's, not the program's.
use crate::domain::physical_plan::types::PlanNodeId;

/// The nodes the open file touched, each once.
#[derive(Debug, Default)]
pub(crate) struct TouchedNodes {
    /// The nodes, in the order the file touched them.
    pub(crate) nodes: Vec<PlanNodeId>,
    /// Whether the file touched each node, by node id.
    pub(crate) marked: Vec<Touch>,
}

/// Whether the open file touched a node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Touch {
    /// It did not.
    #[default]
    Untouched,
    /// It did.
    Touched,
}

impl TouchedNodes {
    /// The open file touched `node`.
    pub(crate) fn touched(&mut self, node: PlanNodeId) {
        let at = usize::try_from(node.0).unwrap_or(usize::MAX);
        if self.marked.len() <= at {
            self.marked.resize(at.saturating_add(1), Touch::Untouched);
        }
        if let Some(marked) = self.marked.get_mut(at)
            && *marked == Touch::Untouched
        {
            *marked = Touch::Touched;
            self.nodes.push(node);
        }
    }

    /// The nodes the file touched, ascending, each untouched again: the next file starts from none.
    pub(crate) fn taken(&mut self) -> Vec<PlanNodeId> {
        let mut nodes = std::mem::take(&mut self.nodes);
        nodes.sort_unstable();
        for node in &nodes {
            if let Some(marked) = self
                .marked
                .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
            {
                *marked = Touch::Untouched;
            }
        }
        nodes
    }

    /// Takes back the list `taken` answered, emptied, for its room.
    pub(crate) fn given_back(&mut self, mut nodes: Vec<PlanNodeId>) {
        nodes.clear();
        nodes.extend(self.nodes.drain(..));
        self.nodes = nodes;
    }
}
