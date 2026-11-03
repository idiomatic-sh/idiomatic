//! Pure traversals feed completed answers to the common resource owner.
use super::super::{
    handles::{GraphAccess, RootAnswer},
    retained::{GraphEntry, NearestMemo, RootMemo},
};
use crate::application::file_dag::derived::{
    backend::CacheBackend,
    graph_cache::{
        service::{replace, slot},
        types::footprint::value::{arc_bytes, vector},
    },
    ledger::Backing,
    owner::DerivedOwner,
};
use crate::domain::{
    operating::errors::OperatingError,
    stored_graph::types::{
        GraphDomain, GraphWalk, Orientation,
        traversal::value::{GraphTraversal, UNKNOWN},
    },
};
use ryo_relations::domain::relations::types::Value;
use std::sync::Arc;

/// The owner supplies both the retained storage and every admission decision.
/// Mutable backend access ends before a downstream row can emit.
pub(in crate::application::file_dag) struct DemandAnswers<'a> {
    owner: &'a DerivedOwner,
    cache: Option<&'a mut CacheBackend>,
}
impl<'a> DemandAnswers<'a> {
    pub(in crate::application::file_dag) fn new(
        owner: &'a DerivedOwner,
        cache: Option<&'a mut CacheBackend>,
    ) -> Self {
        Self { owner, cache }
    }
    /// Only a completed positive closure is retained. Unknown roots and local
    /// traversal scratch never become successful memo records.
    pub(in crate::application::file_dag) fn reach(
        &mut self,
        access: &GraphAccess,
        partition: &[Value],
        root: Value,
        walk: &mut GraphWalk,
    ) -> Result<Option<RootAnswer>, OperatingError> {
        walk.reached.clear();
        walk.expanded = 0;
        walk.examined = 0;
        if let Some(cache) = self.cache.as_mut() {
            cache.graph_counts.roots += 1;
        }
        crate::application::file_dag::route_counts::event("reach_request", "graph");
        let Some(partition) = access.graph.partition(partition) else {
            return Ok(None);
        };
        let Some(root) = access.graph.vertex(partition, root) else {
            return Ok(None);
        };
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
            return completed(self.owner, entry, Arc::clone(&answer.reached)).map(Some);
        }
        crate::application::file_dag::route_counts::event("reach_traversal", "graph");
        let mut traversal = GraphTraversal::new(&access.graph, walk);
        let stopped = traversal.breadth(root, access.orientation, None);
        traversal.sort();
        if let Some(cache) = self.cache.as_mut() {
            cache.graph_counts.traversals += 1;
            cache.graph_counts.expanded += walk.expanded;
            cache.graph_counts.examined += walk.examined;
        }
        // A stopped traversal has not completed the requested closure.
        if stopped || !access.shared {
            return Ok(None);
        }
        self.admit(access, root, walk)
    }
    pub(in crate::application::file_dag) fn nearest(
        &mut self,
        access: &GraphAccess,
        domain: &Arc<GraphDomain>,
        root: u32,
        orientation: Orientation,
        local: &mut Vec<u32>,
        walk: &mut GraphWalk,
    ) -> Result<Option<u32>, OperatingError> {
        crate::application::file_dag::route_counts::event("nearest_request", "graph");
        if access.graph.forests()
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
            return Ok(stopped.then(|| *walk.reached.last().expect("a qualifying encounter")));
        }
        let shared = self.forest_context(access, domain, orientation)?;
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
        if let Some(cache) = self.cache.as_mut() {
            crate::application::file_dag::route_counts::event(
                if hit {
                    "nearest_memo_hit"
                } else {
                    "nearest_traversal"
                },
                "graph",
            );
            cache.graph_counts.forest_hits += u64::from(hit);
            cache.graph_counts.expanded += walk.expanded;
            cache.graph_counts.examined += walk.examined;
        }
        Ok(answer)
    }
    fn admit(
        &mut self,
        access: &GraphAccess,
        root: u32,
        walk: &GraphWalk,
    ) -> Result<Option<RootAnswer>, OperatingError> {
        let Some(cache) = self.cache.as_mut() else {
            return Ok(None);
        };
        let Some(index) = cache
            .graphs
            .iter()
            .position(|entry| Arc::ptr_eq(&entry.graph, &access.graph))
        else {
            return Ok(None);
        };
        let prepared = (|| {
            let minimum = walk
                .reached
                .len()
                .checked_mul(size_of::<u32>())?
                .checked_add(arc_bytes::<Vec<u32>>())?
                .checked_add(size_of::<RootMemo>())?;
            if !self.owner.graph_room(cache, minimum, true) {
                return None;
            }
            let (replacement, metadata) = slot(&cache.graphs[index].roots)?;
            let mut reached = Vec::new();
            reached.try_reserve_exact(walk.reached.len()).ok()?;
            reached.extend_from_slice(&walk.reached);
            let cost = arc_bytes::<Vec<u32>>()
                .checked_add(vector(&reached))?
                .checked_add(metadata)?;
            Some((replacement, cost, Arc::new(reached)))
        })();
        let Some((replacement, cost, reached)) = prepared else {
            return Ok(None);
        };
        // Dense vertex IDs are memo data, not retained physical row positions.
        let backing = Backing {
            key: (20, Arc::as_ptr(&reached) as usize),
            retention_units: 0,
            bytes: arc_bytes::<Vec<u32>>() + vector(&reached),
        };
        let Some(lease) = self.owner.try_graph_lease(cache, &[backing], cost, true)? else {
            return Ok(None);
        };
        replace(&mut cache.graphs[index].roots, replacement);
        cache.graphs[index].roots.push(RootMemo {
            root,
            orientation: access.orientation,
            reached: Arc::clone(&reached),
            _lease: lease,
        });
        cache.bytes += cost;
        cache.graph_bytes += cost;
        cache.memo_bytes += cost;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(cache.bytes);
        completed(self.owner, &cache.graphs[index], reached).map(Some)
    }
    /// Exact graph allocation, exact admitted domain and orientation jointly
    /// define a nearest forest context; no answer is shared on cardinality alone.
    fn forest_context(
        &mut self,
        access: &GraphAccess,
        domain: &Arc<GraphDomain>,
        orientation: Orientation,
    ) -> Result<Option<usize>, OperatingError> {
        let Some(cache) = self.cache.as_mut() else {
            return Ok(None);
        };
        if !access.shared {
            return Ok(None);
        }
        let Some(domain_index) = cache.graph_domains.iter().position(|entry| {
            Arc::ptr_eq(&entry.graph, &access.graph) && Arc::ptr_eq(&entry.domain, domain)
        }) else {
            return Ok(None);
        };
        if let Some(index) = cache.nearest.iter().position(|entry| {
            Arc::ptr_eq(&entry.graph, &access.graph)
                && Arc::ptr_eq(&entry.domain, domain)
                && entry.orientation == orientation
        }) {
            return Ok(Some(index));
        }
        let prepared = (|| {
            let minimum = access
                .graph
                .vertices()
                .len()
                .checked_mul(size_of::<u32>())?
                .checked_add(size_of::<NearestMemo>())?;
            if !self.owner.graph_room(cache, minimum, true) {
                return None;
            }
            let (replacement, metadata) = slot(&cache.nearest)?;
            let mut answers = Vec::new();
            answers
                .try_reserve_exact(access.graph.vertices().len())
                .ok()?;
            answers.resize(access.graph.vertices().len(), UNKNOWN);
            let cost = vector(&answers).checked_add(metadata)?;
            Some((replacement, cost, answers))
        })();
        let Some((replacement, cost, answers)) = prepared else {
            return Ok(None);
        };
        let mut backings = cache.graph_domains[domain_index].backings.to_vec();
        backings.push(Backing {
            key: (21, answers.as_ptr() as usize),
            bytes: vector(&answers),
            retention_units: 0,
        });
        let Some(lease) = self.owner.try_graph_lease(cache, &backings, cost, true)? else {
            return Ok(None);
        };
        replace(&mut cache.nearest, replacement);
        cache.nearest.push(NearestMemo {
            _graph_storage: Arc::clone(&cache.graph_domains[domain_index].graph_storage),
            _domain_storage: Arc::clone(&cache.graph_domains[domain_index].storage),
            graph: Arc::clone(&access.graph),
            domain: Arc::clone(domain),
            orientation,
            answers,
            _lease: lease,
        });
        cache.bytes += cost;
        cache.graph_bytes += cost;
        cache.memo_bytes += cost;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(cache.bytes);
        Ok(Some(cache.nearest.len() - 1))
    }
}

fn completed(
    owner: &DerivedOwner,
    graph: &GraphEntry,
    rows: Arc<Vec<u32>>,
) -> Result<RootAnswer, OperatingError> {
    let mut backings = graph.backings.to_vec();
    backings.push(Backing {
        key: (20, Arc::as_ptr(&rows) as usize),
        bytes: arc_bytes::<Vec<u32>>() + vector(&rows),
        retention_units: 0,
    });
    let lease = owner.required_lease(&backings)?;
    Ok(RootAnswer::new(rows, Arc::clone(&graph.storage), lease))
}
