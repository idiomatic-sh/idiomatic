//! The common owner alone installs retained graph and domain storage.
use super::super::{
    ledger::types::Backing,
    owner::graphs::{
        service::{replace, room, slot},
        types::{
            handles::{service::GraphHandle, types::GraphSharing},
            retained::types::{DomainEntry, GraphEntry},
        },
    },
};
use super::*;
use crate::domain::stored_graph::types::GraphDomain;
use ryo_derived::domain::contract::types::CompactGraph;
impl DerivedOwner {
    pub(in crate::application::file_dag::derived::owner) fn retain_graph(
        &self,
        cache: &mut CacheBackend,
        population: PopulationId,
        storage: &Arc<DerivedStorage>,
        backings: &Arc<[Backing]>,
        graph: &Arc<CompactGraph>,
    ) -> Result<GraphSharing, OperatingError> {
        if cache
            .graphs
            .iter()
            .any(|entry| Arc::ptr_eq(&entry.graph, graph))
        {
            return Ok(GraphSharing::Shared);
        }
        if let Some(reserve) = slot(&cache.graphs) {
            let replacement = reserve.replacement;
            let cost = self.graph_retention_cost(backings, reserve.metadata)?;
            if self.prepare_graph_room(cache, cost, backings)? == Room::Fits
                && let Some(bytes) = room(cache, cost).map(|total| total.0)
                && let Some(lease) =
                    self.try_graph_lease(cache, backings, cost, MemoBudget::Exempt)?
            {
                replace(&mut cache.graphs, replacement);
                cache.graphs.push(GraphEntry {
                    graph: Arc::clone(graph),
                    roots: Vec::new(),
                    population,
                    storage: Arc::clone(storage),
                    backings: Arc::clone(backings),
                    _lease: lease,
                });
                cache.graph_bytes += bytes - cache.bytes;
                cache.bytes = bytes;
                cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
                return Ok(GraphSharing::Shared);
            }
        }
        cache.graph_counts.refusals += 1;
        Ok(GraphSharing::Local)
    }
    pub(in crate::application::file_dag::derived::owner) fn retain_domain(
        &self,
        cache: &mut CacheBackend,
        population: PopulationId,
        graph: &GraphHandle,
        domain: &Arc<GraphDomain>,
        storage: &Arc<DerivedStorage>,
        backings: &Arc<[Backing]>,
    ) -> Result<GraphSharing, OperatingError> {
        if graph.sharing() == GraphSharing::Local || domain.identity.is_none() {
            return Ok(GraphSharing::Local);
        }
        if cache.graph_domains.iter().any(|entry| {
            Arc::ptr_eq(&entry.graph, graph.raw_graph()) && Arc::ptr_eq(&entry.domain, domain)
        }) {
            return Ok(GraphSharing::Shared);
        }
        if let Some(reserve) = slot(&cache.graph_domains) {
            let replacement = reserve.replacement;
            let cost = self.graph_retention_cost(backings, reserve.metadata)?;
            if let Some(bytes) = room(cache, cost).map(|total| total.0)
                && let Some(lease) =
                    self.try_graph_lease(cache, backings, cost, MemoBudget::Exempt)?
            {
                replace(&mut cache.graph_domains, replacement);
                cache.graph_domains.push(DomainEntry {
                    graph_storage: Arc::clone(graph.acquired().storage()),
                    graph: Arc::clone(graph.raw_graph()),
                    domain: Arc::clone(domain),
                    population,
                    storage: Arc::clone(storage),
                    backings: Arc::clone(backings),
                    _lease: lease,
                });
                cache.graph_bytes += bytes - cache.bytes;
                cache.bytes = bytes;
                cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
                return Ok(GraphSharing::Shared);
            }
        }
        Ok(GraphSharing::Local)
    }
}
