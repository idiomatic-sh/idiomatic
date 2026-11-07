//! Pure traversals feed completed answers to the common resource owner.
use ryo_derived::domain::vertices::types::VertexId;
use super::super::{
    handles::{
        service::{DomainHandle, GraphHandle, RootAnswer},
        types::GraphSharing,
    },
    reserve::types::Reserve,
    retained::types::{
        GraphEntry, NearestMemo, NearestSlot, ReachedVertices,
        RootMemo,
    },
};
use super::completed::service::completed;
use crate::application::file_dag::derived::{
    constants,
    ledger::{
        capacity::{arc_bytes, vector},
        types::{Backing, BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount},
    },
    owner::{
        DerivedOwner,
        cache::types::CacheBackend,
        graphs::service::{replace, slot},
    },
};
use crate::domain::{
    operating::errors::OperatingError,
    stored_graph::types::{BreadthEnd, ExaminedVertices, ExpandedVertices, GraphDomain, GraphWalk, MemoStatus, Nearest, Orientation, traversal::value::{GraphTraversal, UNKNOWN},
    },
};
use ryo_relations::domain::relations::types::Value;
use std::sync::Arc;

/// The owner supplies both the retained storage and every admission decision.
/// Mutable backend access ends before a downstream row can emit.
pub(in crate::application::file_dag) struct DemandAnswers<'a> {
    owner: &'a DerivedOwner,
    cache: &'a mut CacheBackend,
}
impl<'a> DemandAnswers<'a> {
    pub(in crate::application::file_dag) fn new(
        owner: &'a DerivedOwner,
        cache: &'a mut CacheBackend,
    ) -> Self {
        Self { owner, cache }
    }
    /// Only a completed positive closure is retained. Unknown roots and local
    /// traversal scratch never become successful memo records.
    pub(in crate::application::file_dag) fn reach(
        &mut self,
        access: &GraphHandle,
        partition: &[Value],
        root: Value,
        walk: &mut GraphWalk,
    ) -> Result<Option<RootAnswer>, OperatingError> {
        access.validate_owner(self.owner)?;
        walk.reached.clear();
        walk.expanded = ExpandedVertices(0);
        walk.examined = ExaminedVertices(0);
        {
            let cache = &mut *self.cache;
            cache.graph_counts.roots.0 += 1;
        }
        crate::application::file_dag::route_counts::event(
            constants::REACH_REQUEST,
            constants::ROUTE_GRAPH,
        );
        let Some(partition) = access.graph().partition(partition) else {
            return Ok(None);
        };
        let Some(root) = access
            .graph()
            .vertex(partition, root)
            .map(|vertex| vertex.0)
        else {
            return Ok(None);
        };
        let cache = &mut *self.cache;
        if access.sharing() == GraphSharing::Shared
            && let Some(entry) = cache
                .graphs
                .iter()
                .find(|entry| Arc::ptr_eq(&entry.graph, access.raw_graph()))
            && let Some(answer) = entry
                .roots
                .iter()
                .find(|answer| answer.root.0 == root && answer.orientation == access.orientation())
        {
            crate::application::file_dag::route_counts::event(
                constants::REACH_MEMO_HIT,
                constants::ROUTE_GRAPH,
            );
            cache.graph_counts.memo_hits.0 += 1;
            return completed(self.owner, entry, Arc::clone(&answer.reached)).map(Some);
        }
        crate::application::file_dag::route_counts::event(
            constants::REACH_TRAVERSAL,
            constants::ROUTE_GRAPH,
        );
        let mut traversal = GraphTraversal::new(access.raw_graph(), walk);
        let stopped = traversal.breadth(VertexId(root), access.orientation(), None)
            == BreadthEnd::Stopped;
        traversal.sort();
        {
            let cache = &mut *self.cache;
            cache.graph_counts.traversals.0 += 1;
            cache.graph_counts.expanded.0 += walk.expanded.0;
            cache.graph_counts.examined.0 += walk.examined.0;
        }
        // A stopped traversal has not completed the requested closure.
        if stopped || access.sharing() == GraphSharing::Local {
            return Ok(None);
        }
        self.admit(access, VertexId(root), walk)
    }
    /// One-step or closure traversal for an axis row; the owner alone records its walk work.
    pub(in crate::application::file_dag) fn axis(
        &mut self,
        access: &GraphHandle,
        root: VertexId,
        orientation: Orientation,
        direction: ryo_values::domain::plan_parts::types::Direction,
        walk: &mut GraphWalk,
    ) -> Result<(), OperatingError> {
        access.validate_owner(self.owner)?;
        let mut traversal = GraphTraversal::new(access.raw_graph(), walk);
        if matches!(
            direction,
            ryo_values::domain::plan_parts::types::Direction::Parent
                | ryo_values::domain::plan_parts::types::Direction::Child
        ) {
            traversal.direct(root, orientation);
        } else {
            let _ = traversal.breadth(root, orientation, None);
        }
        {
            let cache = &mut *self.cache;
            cache.observe_graph_walk(walk.expanded, walk.examined);
        }
        Ok(())
    }
    pub(in crate::application::file_dag) fn nearest(
        &mut self,
        access: &GraphHandle,
        domain: &DomainHandle,
        root: VertexId,
        orientation: Orientation,
        local: &mut Vec<VertexId>,
        walk: &mut GraphWalk,
    ) -> Result<Option<VertexId>, OperatingError> {
        access.validate_owner(self.owner)?;
        domain.validate_owner(self.owner, access)?;
        let domain = domain.raw_domain();
        crate::application::file_dag::route_counts::event(
            constants::NEAREST_REQUEST,
            constants::ROUTE_GRAPH,
        );
        if access
            .graph()
            .forest(crate::domain::stored_graph::types::traversal::value::direction(orientation).0)
            .is_none()
        {
            crate::application::file_dag::route_counts::event(
                constants::NEAREST_TRAVERSAL,
                constants::ROUTE_GRAPH,
            );
            let stopped = GraphTraversal::new(access.raw_graph(), walk).breadth(
                root,
                orientation,
                Some(domain.as_ref()),
            ) == BreadthEnd::Stopped;
            {
                let cache = &mut *self.cache;
                cache.graph_counts.early_stops.0 += u64::from(stopped);
                cache.graph_counts.expanded.0 += walk.expanded.0;
                cache.graph_counts.examined.0 += walk.examined.0;
            }
            return Ok(stopped.then(|| {
                *walk
                    .reached
                    .last()
                    .expect(constants::EXPECT_QUALIFYING_ENCOUNTER)
            }));
        }
        let shared = self.forest_context(access, domain, orientation)?;
        let Nearest {
            vertex: answer,
            memo,
        } = if let Some(index) = shared {
            GraphTraversal::new(access.raw_graph(), walk).nearest(
                root,
                orientation,
                domain,
                &mut self.cache.nearest[index.0].answers,
            )
        } else {
            GraphTraversal::new(access.raw_graph(), walk).nearest(
                root,
                orientation,
                domain,
                local,
            )
        };
        let hit = memo == MemoStatus::Memoized;
        {
            let cache = &mut *self.cache;
            crate::application::file_dag::route_counts::event(
                if hit {
                    constants::NEAREST_MEMO_HIT
                } else {
                    constants::NEAREST_TRAVERSAL
                },
                constants::ROUTE_GRAPH,
            );
            cache.graph_counts.forest_hits.0 += u64::from(hit);
            cache.graph_counts.expanded.0 += walk.expanded.0;
            cache.graph_counts.examined.0 += walk.examined.0;
        }
        Ok(answer)
    }
    fn admit(
        &mut self,
        access: &GraphHandle,
        root: VertexId,
        walk: &GraphWalk,
    ) -> Result<Option<RootAnswer>, OperatingError> {
        let cache = &mut *self.cache;
        let Some(index) = cache
            .graphs
            .iter()
            .position(|entry| Arc::ptr_eq(&entry.graph, access.raw_graph()))
        else {
            return Ok(None);
        };
        let prepared = (|| {
            let minimum = walk
                .reached
                .len()
                .checked_mul(size_of::<u32>())?
                .checked_add(arc_bytes::<Vec<u32>>().0)?
                .checked_add(size_of::<RootMemo>())?;
            if self.owner.graph_room(
                cache,
                ByteCount(minimum),
                crate::application::file_dag::derived::owner::types::MemoBudget::Charged,
            ) == crate::application::file_dag::derived::owner::types::Room::Full
            {
                return None;
            }
            let Reserve {
                replacement,
                metadata,
            } = slot(&cache.graphs[index].roots)?;
            let metadata = metadata.0;
            let mut reached = Vec::new();
            if reached.try_reserve_exact(walk.reached.len()).is_err() {
                return None;
            }
            reached.extend(walk.reached.iter().map(|vertex| vertex.0));
            let cost = arc_bytes::<Vec<u32>>()
                .0
                .checked_add(vector(&reached).0)?
                .checked_add(metadata)?;
            Some((replacement, cost, Arc::new(ReachedVertices(reached))))
        })();
        let Some((replacement, cost, reached)) = prepared else {
            return Ok(None);
        };
        // Dense vertex IDs are memo data, not retained physical row positions.
        let backing = Backing {
            key: BackingKey {
                tag: BackingTag(20),
                address: BackingAddress(Arc::as_ptr(&reached) as usize),
            },
            retention_units: UnitCount(0),
            bytes: ByteCount(arc_bytes::<Vec<u32>>().0 + vector(&reached.0).0),
        };
        let Some(lease) = self.owner.try_graph_lease(
            cache,
            &[backing],
            ByteCount(cost),
            crate::application::file_dag::derived::owner::types::MemoBudget::Charged,
        )?
        else {
            return Ok(None);
        };
        replace(&mut cache.graphs[index].roots, replacement);
        cache.graphs[index].roots.push(RootMemo {
            root,
            orientation: access.orientation(),
            reached: Arc::clone(&reached),
            _lease: lease,
        });
        cache.bytes.0 += cost;
        cache.graph_bytes.0 += cost;
        cache.memo_bytes.0 += cost;
        cache.graph_counts.peak_bytes.0 = cache.graph_counts.peak_bytes.0.max(cache.bytes.0);
        completed(self.owner, &cache.graphs[index], reached).map(Some)
    }
    /// Exact graph allocation, exact admitted domain and orientation jointly
    /// define a nearest forest context; no answer is shared on cardinality alone.
    fn forest_context(
        &mut self,
        access: &GraphHandle,
        domain: &Arc<GraphDomain>,
        orientation: Orientation,
    ) -> Result<Option<NearestSlot>, OperatingError> {
        let cache = &mut *self.cache;
        if access.sharing() == GraphSharing::Local {
            return Ok(None);
        }
        let Some(domain_index) = cache.graph_domains.iter().position(|entry| {
            Arc::ptr_eq(&entry.graph, access.raw_graph()) && Arc::ptr_eq(&entry.domain, domain)
        }) else {
            return Ok(None);
        };
        if let Some(index) = cache.nearest.iter().position(|entry| {
            Arc::ptr_eq(&entry.graph, access.raw_graph())
                && Arc::ptr_eq(&entry.domain, domain)
                && entry.orientation == orientation
        }) {
            return Ok(Some(NearestSlot(index)));
        }
        let prepared = (|| {
            let minimum = access
                .graph()
                .vertices()
                .len()
                .checked_mul(size_of::<u32>())?
                .checked_add(size_of::<NearestMemo>())?;
            if self.owner.graph_room(
                cache,
                ByteCount(minimum),
                crate::application::file_dag::derived::owner::types::MemoBudget::Charged,
            ) == crate::application::file_dag::derived::owner::types::Room::Full
            {
                return None;
            }
            let Reserve {
                replacement,
                metadata,
            } = slot(&cache.nearest)?;
            let metadata = metadata.0;
            let mut answers = Vec::new();
            if answers
                .try_reserve_exact(access.graph().vertices().len())
                .is_err()
            {
                return None;
            }
            answers.resize(access.graph().vertices().len(), UNKNOWN);
            let cost = vector(&answers).0.checked_add(metadata)?;
            Some((replacement, cost, answers))
        })();
        let Some((replacement, cost, answers)) = prepared else {
            return Ok(None);
        };
        let mut backings = cache.graph_domains[domain_index].backings.to_vec();
        backings.push(Backing {
            key: BackingKey {
                tag: BackingTag(21),
                address: BackingAddress(answers.as_ptr() as usize),
            },
            bytes: vector(&answers),
            retention_units: UnitCount(0),
        });
        let Some(lease) = self.owner.try_graph_lease(
            cache,
            &backings,
            ByteCount(cost),
            crate::application::file_dag::derived::owner::types::MemoBudget::Charged,
        )?
        else {
            return Ok(None);
        };
        replace(&mut cache.nearest, replacement);
        cache.nearest.push(NearestMemo {
            _graph_storage: Arc::clone(&cache.graph_domains[domain_index].graph_storage),
            _domain_storage: Arc::clone(&cache.graph_domains[domain_index].storage),
            graph: Arc::clone(access.raw_graph()),
            domain: Arc::clone(domain),
            orientation,
            answers,
            _lease: lease,
        });
        cache.bytes.0 += cost;
        cache.graph_bytes.0 += cost;
        cache.memo_bytes.0 += cost;
        cache.graph_counts.peak_bytes.0 = cache.graph_counts.peak_bytes.0.max(cache.bytes.0);
        Ok(Some(NearestSlot(cache.nearest.len() - 1)))
    }
}
