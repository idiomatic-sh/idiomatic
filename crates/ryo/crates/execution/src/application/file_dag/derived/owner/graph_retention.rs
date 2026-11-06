//! The common owner alone installs retained graph and domain storage.
use super::super::{
    ledger::Backing,
    owner::graphs::{
        service::{replace, room, slot},
        types::{
            handles::GraphHandle,
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
    ) -> Result<bool, OperatingError> {
        if cache
            .graphs
            .iter()
            .any(|entry| Arc::ptr_eq(&entry.graph, graph))
        {
            return Ok(true);
        }
        if let Some((replacement, metadata)) = slot(&cache.graphs) {
            let cost = self.graph_retention_cost(backings, metadata)?;
            if self.prepare_graph_room(cache, cost, backings)?
                && let Some(bytes) = room(cache, cost)
                && let Some(lease) = self.try_graph_lease(cache, backings, cost, false)?
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
                return Ok(true);
            }
        }
        cache.graph_counts.refusals += 1;
        Ok(false)
    }
    pub(in crate::application::file_dag::derived::owner) fn retain_domain(
        &self,
        cache: &mut CacheBackend,
        population: PopulationId,
        graph: &GraphHandle,
        domain: &Arc<GraphDomain>,
        storage: &Arc<DerivedStorage>,
        backings: &Arc<[Backing]>,
    ) -> Result<bool, OperatingError> {
        if !graph.shared() || domain.identity.is_none() {
            return Ok(false);
        }
        if cache.graph_domains.iter().any(|entry| {
            Arc::ptr_eq(&entry.graph, graph.raw_graph()) && Arc::ptr_eq(&entry.domain, domain)
        }) {
            return Ok(true);
        }
        if let Some((replacement, metadata)) = slot(&cache.graph_domains) {
            let cost = self.graph_retention_cost(backings, metadata)?;
            if let Some(bytes) = room(cache, cost)
                && let Some(lease) = self.try_graph_lease(cache, backings, cost, false)?
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
                return Ok(true);
            }
        }
        Ok(false)
    }
}
