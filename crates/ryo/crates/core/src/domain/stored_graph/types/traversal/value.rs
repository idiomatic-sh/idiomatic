//! Iterative positive reach, ordered early stop and validated-forest path compression.

use crate::domain::stored_graph::types::{GraphDomain, GraphWalk, Orientation};
use ryo_derived::domain::contract::types::CompactGraph;

/// No known nearest answer has been computed for this vertex/context.
pub(crate) const UNKNOWN: u32 = u32::MAX;
/// A complete parent-chain walk found no qualifying ancestor.
pub(crate) const ABSENT: u32 = u32::MAX - 1;

/// A read-only graph plus reusable invocation-owned traversal memory.
pub(crate) struct GraphTraversal<'a> {
    /// Complete, immutable adjacency.
    graph: &'a CompactGraph,
    /// No provider references or mutable file-cache borrow.
    walk: &'a mut GraphWalk,
}

impl<'a> GraphTraversal<'a> {
    /// Borrow no provider or cache state while traversing.
    pub(crate) fn new(graph: &'a CompactGraph, walk: &'a mut GraphWalk) -> Self {
        Self { graph, walk }
    }

    /// Direct neighbors retain physical duplicate edges, as Parent/Child axes do.
    pub(crate) fn direct(&mut self, root: u32, orientation: Orientation) {
        self.begin();
        self.walk
            .reached
            .extend_from_slice(neighbors(self.graph, root, orientation));
        self.walk.expanded = 1;
        self.walk.examined = self.walk.reached.len() as u64;
    }

    /// Positive-length BFS, optionally stopping before expansion of its first qualifying endpoint.
    pub(crate) fn breadth(
        &mut self,
        root: u32,
        orientation: Orientation,
        nearest: Option<&GraphDomain>,
    ) -> bool {
        self.begin();
        self.follow(root, orientation);
        while let Some(vertex) = self.walk.queue.pop_front() {
            self.walk.reached.push(vertex);
            if nearest.is_some_and(|domain| domain.members.get(vertex as usize) == Some(&true)) {
                return true;
            }
            // Initial seeding already expanded the root. A real cycle may still emit it.
            if vertex != root {
                self.follow(vertex, orientation);
            }
        }
        false
    }

    /// Closure destinations sort by typed Value, independently of Axis encounter order.
    pub(crate) fn sort(&mut self) {
        self.walk
            .reached
            .sort_unstable_by_key(|vertex| self.graph.vertices()[*vertex as usize]);
    }

    /// Path-compressed nearest ancestry in a previously validated acyclic orientation.
    pub(crate) fn nearest(
        &mut self,
        root: u32,
        orientation: Orientation,
        domain: &GraphDomain,
        answers: &mut Vec<u32>,
    ) -> (Option<u32>, bool) {
        self.walk.expanded = 0;
        self.walk.examined = 0;
        self.walk.path.clear();
        let Some(parents) = &self.graph.forests()[direction(orientation)] else {
            return (None, false);
        };
        answers.resize(self.graph.vertices().len(), UNKNOWN);
        let known = answers[root as usize];
        if known != UNKNOWN {
            return ((known != ABSENT).then_some(known), true);
        }
        let mut current = root;
        let answer = loop {
            self.walk.path.push(current);
            self.walk.expanded += 1;
            let parent = parents[current as usize];
            if parent == ABSENT {
                break ABSENT;
            }
            self.walk.examined += 1;
            if domain.members[parent as usize] {
                break parent;
            }
            if answers[parent as usize] != UNKNOWN {
                break answers[parent as usize];
            }
            current = parent;
        };
        for vertex in &self.walk.path {
            answers[*vertex as usize] = answer;
        }
        ((answer != ABSENT).then_some(answer), false)
    }

    /// Epochs avoid clearing every vertex on each narrow source request.
    fn begin(&mut self) {
        if self.walk.marks.len() < self.graph.vertices().len() {
            self.walk.marks.resize(self.graph.vertices().len(), 0);
        }
        self.walk.epoch = self.walk.epoch.wrapping_add(1);
        if self.walk.epoch == 0 {
            self.walk.marks.fill(0);
            self.walk.epoch = 1;
        }
        self.walk.queue.clear();
        self.walk.reached.clear();
        self.walk.expanded = 0;
        self.walk.examined = 0;
    }

    /// Expand each vertex once, including a root later reached through a real cycle.
    fn follow(&mut self, vertex: u32, orientation: Orientation) {
        self.walk.expanded += 1;
        for neighbor in neighbors(self.graph, vertex, orientation) {
            self.walk.examined += 1;
            let mark = &mut self.walk.marks[*neighbor as usize];
            if *mark != self.walk.epoch {
                *mark = self.walk.epoch;
                self.walk.queue.push_back(*neighbor);
            }
        }
    }
}

/// Dense adjacency stores forward then reverse arrays.
pub(crate) const fn direction(orientation: Orientation) -> usize {
    match orientation {
        Orientation::Forward => 0,
        Orientation::Reverse => 1,
    }
}

/// Original encounter order, including parallel physical edges.
pub(crate) fn neighbors(graph: &CompactGraph, vertex: u32, orientation: Orientation) -> &[u32] {
    let orientation = direction(orientation);
    let from = graph.offsets()[orientation][vertex as usize] as usize;
    let until = graph.offsets()[orientation][vertex as usize + 1] as usize;
    &graph.neighbors()[orientation][from..until]
}
