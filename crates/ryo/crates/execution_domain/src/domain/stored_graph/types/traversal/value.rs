//! Iterative positive reach, ordered early stop and validated-forest path compression.

use crate::domain::stored_graph::types::{BreadthEnd, ExaminedVertices, ExpandedVertices, GraphDomain, GraphWalk, MemoStatus, Nearest, Neighbors, Orientation, VisitEpoch};
use ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph;
use ryo_derived::domain::vertices::types::VertexId;

/// No known nearest answer has been computed for this vertex/context.
pub const UNKNOWN: VertexId = VertexId(u32::MAX);
/// A complete parent-chain walk found no qualifying ancestor.
pub(crate) const ABSENT: VertexId = VertexId(u32::MAX - 1);

/// A read-only graph plus reusable invocation-owned traversal memory.
#[derive(Debug)]
pub struct GraphTraversal<'a> {
    /// Complete, immutable adjacency.
    graph: &'a CompactGraph,
    /// No provider references or mutable file-cache borrow.
    walk: &'a mut GraphWalk,
}

impl<'a> GraphTraversal<'a> {
    /// Borrow no provider or cache state while traversing.
    pub fn new(graph: &'a CompactGraph, walk: &'a mut GraphWalk) -> Self {
        Self { graph, walk }
    }

    /// Direct neighbors retain physical duplicate edges, as Parent/Child axes do.
    pub fn direct(&mut self, root: VertexId, orientation: Orientation) {
        self.begin();
        self.walk.reached.extend(
            neighbors(self.graph, root, orientation)
                .0
                .iter()
                .copied()
                .map(VertexId),
        );
        self.walk.expanded = ExpandedVertices(1);
        self.walk.examined = ExaminedVertices(self.walk.reached.len() as u64);
    }

    /// Positive-length BFS, optionally stopping before expansion of its first qualifying endpoint.
    pub fn breadth(
        &mut self,
        root: VertexId,
        orientation: Orientation,
        nearest: Option<&GraphDomain>,
    ) -> BreadthEnd {
        self.begin();
        self.follow(root, orientation);
        while let Some(vertex) = self.walk.queue.pop_front() {
            self.walk.reached.push(vertex);
            if nearest.is_some_and(|domain| domain.members.0.as_slice().get(vertex.0 as usize) == Some(&true)) {
                return BreadthEnd::Stopped;
            }
            // Initial seeding already expanded the root. A real cycle may still emit it.
            if vertex != root {
                self.follow(vertex, orientation);
            }
        }
        BreadthEnd::Complete
    }

    /// Closure destinations sort by typed Value, independently of Axis encounter order.
    pub fn sort(&mut self) {
        self.walk
            .reached
            .sort_unstable_by_key(|vertex| self.graph.vertices()[vertex.0 as usize]);
    }

    /// Path-compressed nearest ancestry in a previously validated acyclic orientation.
    pub fn nearest(
        &mut self,
        root: VertexId,
        orientation: Orientation,
        domain: &GraphDomain,
        answers: &mut Vec<VertexId>,
    ) -> Nearest {
        self.walk.expanded = ExpandedVertices(0);
        self.walk.examined = ExaminedVertices(0);
        self.walk.path.clear();
        let Some(parents) = self.graph.forest(direction(orientation)) else {
            return Nearest {
                vertex: None,
                memo: MemoStatus::Fresh,
            };
        };
        answers.resize(self.graph.vertices().len(), UNKNOWN);
        let known = answers[root.0 as usize];
        if known != UNKNOWN {
            return Nearest {
                vertex: (known != ABSENT).then_some(known),
                memo: MemoStatus::Memoized,
            };
        }
        let mut current = root;
        let answer = loop {
            self.walk.path.push(current);
            self.walk.expanded.0 += 1;
            let Some(parent) = parents
                .parent(ryo_derived::domain::vertices::types::VertexId(current.0))
                .map(|parent| parent.0)
            else {
                break ABSENT;
            };
            self.walk.examined.0 += 1;
            let parent = VertexId(parent);
            if domain.members.0.as_slice()[parent.0 as usize] {
                break parent;
            }
            if answers[parent.0 as usize] != UNKNOWN {
                break answers[parent.0 as usize];
            }
            current = parent;
        };
        for vertex in &self.walk.path {
            answers[vertex.0 as usize] = answer;
        }
        Nearest {
            vertex: (answer != ABSENT).then_some(answer),
            memo: MemoStatus::Fresh,
        }
    }

    /// Epochs avoid clearing every vertex on each narrow source request.
    fn begin(&mut self) {
        if self.walk.marks.len() < self.graph.vertices().len() {
            self.walk.marks.resize(self.graph.vertices().len(), VisitEpoch(0));
        }
        self.walk.epoch = VisitEpoch(self.walk.epoch.0.wrapping_add(1));
        if self.walk.epoch == VisitEpoch(0) {
            self.walk.marks.fill(VisitEpoch(0));
            self.walk.epoch = VisitEpoch(1);
        }
        self.walk.queue.clear();
        self.walk.reached.clear();
        self.walk.expanded = ExpandedVertices(0);
        self.walk.examined = ExaminedVertices(0);
    }

    /// Expand each vertex once, including a root later reached through a real cycle.
    fn follow(&mut self, vertex: VertexId, orientation: Orientation) {
        self.walk.expanded.0 += 1;
        for neighbor in neighbors(self.graph, vertex, orientation).0 {
            self.walk.examined.0 += 1;
            let mark = &mut self.walk.marks[*neighbor as usize];
            if *mark != self.walk.epoch {
                *mark = self.walk.epoch;
                self.walk.queue.push_back(VertexId(*neighbor));
            }
        }
    }
}

/// Dense adjacency stores forward then reverse arrays.
pub const fn direction(
    orientation: Orientation,
) -> ryo_derived::domain::vertices::types::EdgeDirection {
    match orientation {
        Orientation::Forward => ryo_derived::domain::vertices::types::EdgeDirection::Forward,
        Orientation::Reverse => ryo_derived::domain::vertices::types::EdgeDirection::Backward,
    }
}

/// Original encounter order, including parallel physical edges.
pub(crate) fn neighbors(
    graph: &CompactGraph,
    vertex: VertexId,
    orientation: Orientation,
) -> Neighbors<'_> {
    let orientation = direction(orientation);
    let offsets = &graph.offsets(orientation).0;
    let from = offsets[vertex.0 as usize] as usize;
    let until = offsets[vertex.0 as usize + 1] as usize;
    Neighbors(&graph.neighbors(orientation).0[from..until])
}
