//! Successful demand answers and exact-domain forest memoization under the file's existing cap.

use crate::application::file_dag::derived::graph_cache::service::{
    MEMO_LIMIT, replace, room, slot,
};
use crate::domain::stored_graph::types::footprint::value::{arc_bytes, vector};
use crate::domain::stored_graph::types::traversal::value::{GraphTraversal, UNKNOWN};
use crate::domain::stored_graph::types::{
    GraphAccess, GraphDomain, GraphWalk, NearestMemo, Orientation, RootMemo,
};
use crate::domain::stored_indexes::types::FileIndexes;
use ryo_language::domain::relations::types::Value;
use std::sync::Arc;

/// One generation's mutable memo access; every returned handle releases this borrow before emission.
pub(crate) struct DemandAnswers<'a> {
    /// Optional sharing never owns the provider population.
    cache: Option<&'a mut FileIndexes>,
}

impl<'a> DemandAnswers<'a> {
    /// Borrow the same bounded owner used for graph/index/selector admission.
    pub(crate) fn new(cache: Option<&'a mut FileIndexes>) -> Self {
        Self { cache }
    }

    /// A completed sorted positive-closure answer, shared or in `walk.reached` on local fallback.
    pub(crate) fn reach(
        &mut self,
        access: &GraphAccess,
        partition: &[Value],
        root: Value,
        walk: &mut GraphWalk,
    ) -> Option<Arc<Vec<u32>>> {
        walk.reached.clear();
        walk.expanded = 0;
        walk.examined = 0;
        if let Some(cache) = self.cache.as_mut() {
            cache.graph_counts.roots += 1;
        }
        crate::application::file_dag::route_counts::event("reach_request", "graph");
        let partition = access.graph.partition_ids.get(partition)?;
        let root = *access.graph.vertex_ids.get(&(*partition, root))?;
        if access.shared
            && let Some(cache) = self.cache.as_mut()
            && let Some(entry) = cache
                .graphs
                .iter()
                .find(|entry| Arc::ptr_eq(&entry.graph, &access.graph))
            && let Some(answer) = entry
                .roots
                .iter()
                .find(|answer| answer.root == root && answer.orientation == access.orientation)
        {
            crate::application::file_dag::route_counts::event("reach_memo_hit", "graph");
            cache.graph_counts.memo_hits += 1;
            return Some(Arc::clone(&answer.reached));
        }
        crate::application::file_dag::route_counts::event("reach_traversal", "graph");
        let mut traversal = GraphTraversal::new(&access.graph, walk);
        let _ = traversal.breadth(root, access.orientation, None);
        traversal.sort();
        if let Some(cache) = self.cache.as_mut() {
            cache.graph_counts.traversals += 1;
            cache.graph_counts.expanded += walk.expanded;
            cache.graph_counts.examined += walk.examined;
        }
        if !access.shared {
            return None;
        }
        self.admit(access, root, walk)
    }

    /// Strict nearest ancestry; generic graphs preserve BFS encounter order and stop at first match.
    pub(crate) fn nearest(
        &mut self,
        access: &GraphAccess,
        domain: &Arc<GraphDomain>,
        root: u32,
        orientation: Orientation,
        local: &mut Vec<u32>,
        walk: &mut GraphWalk,
    ) -> Option<u32> {
        crate::application::file_dag::route_counts::event("nearest_request", "graph");
        if access.graph.forests
            [crate::domain::stored_graph::types::traversal::value::direction(orientation)]
        .is_none()
        {
            crate::application::file_dag::route_counts::event("nearest_traversal", "graph");
            let stopped =
                GraphTraversal::new(&access.graph, walk).breadth(root, orientation, Some(domain));
            if let Some(cache) = self.cache.as_mut() {
                cache.graph_counts.early_stops += u64::from(stopped);
                cache.graph_counts.expanded += walk.expanded;
                cache.graph_counts.examined += walk.examined;
            }
            return stopped.then(|| *walk.reached.last().expect("a qualifying encounter"));
        }
        let shared = self.forest_context(access, domain, orientation);
        let (answer, hit) = if let (Some(index), Some(cache)) = (shared, self.cache.as_mut()) {
            GraphTraversal::new(&access.graph, walk).nearest(
                root,
                orientation,
                domain,
                &mut cache.nearest[index].answers,
            )
        } else {
            GraphTraversal::new(&access.graph, walk).nearest(root, orientation, domain, local)
        };
        crate::application::file_dag::route_counts::event(
            if hit {
                "nearest_memo_hit"
            } else {
                "nearest_traversal"
            },
            "graph",
        );
        if let Some(cache) = self.cache.as_mut() {
            cache.graph_counts.forest_hits += u64::from(hit);
            cache.graph_counts.expanded += walk.expanded;
            cache.graph_counts.examined += walk.examined;
        }
        answer
    }

    /// Do not copy a large result merely to discover that the bounded memo cannot retain it.
    fn admit(
        &mut self,
        access: &GraphAccess,
        root: u32,
        walk: &GraphWalk,
    ) -> Option<Arc<Vec<u32>>> {
        let cache = self.cache.as_mut()?;
        let index = cache
            .graphs
            .iter()
            .position(|entry| Arc::ptr_eq(&entry.graph, &access.graph))?;
        let minimum = walk
            .reached
            .len()
            .checked_mul(size_of::<u32>())?
            .checked_add(arc_bytes::<Vec<u32>>())?
            .checked_add(size_of::<RootMemo>())?;
        if cache
            .memo_bytes
            .checked_add(minimum)
            .is_none_or(|bytes| bytes > MEMO_LIMIT)
            || cache
                .bytes
                .checked_add(minimum)
                .is_none_or(|bytes| bytes > cache.limit)
        {
            return None;
        }
        let (replacement, metadata) = slot(&cache.graphs[index].roots)?;
        let mut reached = Vec::new();
        reached.try_reserve_exact(walk.reached.len()).ok()?;
        reached.extend_from_slice(&walk.reached);
        let cost = arc_bytes::<Vec<u32>>()
            .checked_add(vector(&reached))?
            .checked_add(metadata)?;
        if cache
            .memo_bytes
            .checked_add(cost)
            .is_none_or(|bytes| bytes > MEMO_LIMIT)
            || room(cache, cost).is_none()
        {
            return None;
        }
        let reached = Arc::new(reached);
        replace(&mut cache.graphs[index].roots, replacement);
        cache.graphs[index].roots.push(RootMemo {
            root,
            orientation: access.orientation,
            reached: Arc::clone(&reached),
        });
        cache.bytes += cost;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(cache.bytes);
        cache.graph_bytes += cost;
        cache.memo_bytes += cost;
        Some(reached)
    }

    /// Shared nearest arrays require an admitted graph AND the exact admitted domain context.
    fn forest_context(
        &mut self,
        access: &GraphAccess,
        domain: &Arc<GraphDomain>,
        orientation: Orientation,
    ) -> Option<usize> {
        let cache = self.cache.as_mut()?;
        if !access.shared
            || !cache.graph_domains.iter().any(|entry| {
                Arc::ptr_eq(&entry.graph, &access.graph) && Arc::ptr_eq(&entry.domain, domain)
            })
        {
            return None;
        }
        if let Some(index) = cache.nearest.iter().position(|entry| {
            Arc::ptr_eq(&entry.graph, &access.graph)
                && Arc::ptr_eq(&entry.domain, domain)
                && entry.orientation == orientation
        }) {
            return Some(index);
        }
        let minimum = access
            .graph
            .vertices
            .len()
            .checked_mul(size_of::<u32>())?
            .checked_add(size_of::<NearestMemo>())?;
        if cache
            .memo_bytes
            .checked_add(minimum)
            .is_none_or(|bytes| bytes > MEMO_LIMIT)
            || cache
                .bytes
                .checked_add(minimum)
                .is_none_or(|bytes| bytes > cache.limit)
        {
            return None;
        }
        let (replacement, metadata) = slot(&cache.nearest)?;
        let mut answers = Vec::new();
        answers
            .try_reserve_exact(access.graph.vertices.len())
            .ok()?;
        answers.resize(access.graph.vertices.len(), UNKNOWN);
        let cost = vector(&answers).checked_add(metadata)?;
        if cache
            .memo_bytes
            .checked_add(cost)
            .is_none_or(|bytes| bytes > MEMO_LIMIT)
            || room(cache, cost).is_none()
        {
            return None;
        }
        replace(&mut cache.nearest, replacement);
        cache.nearest.push(NearestMemo {
            graph: Arc::clone(&access.graph),
            domain: Arc::clone(domain),
            orientation,
            answers,
        });
        cache.bytes += cost;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(cache.bytes);
        cache.graph_bytes += cost;
        cache.memo_bytes += cost;
        Some(cache.nearest.len() - 1)
    }
}
