//! Graph preparation and row memo acquisition are scoped inside the same owner.
use super::*;
use crate::application::file_dag::derived::backend::CacheBackend;
use crate::{
    application::file_dag::{cached_graph::CachedGraph, workspace::InvocationWorkspace},
    domain::{
        operating::types::OperatorContext,
        operator::contract::{
            graph_view::CachedGraphRow,
            resources::ResourceProgram,
            types::{EvaluationMode, OperatorContract},
        },
        table_model::types::Rows,
    },
};
use ryo_language::domain::relations::types::Value;
impl DerivedOwner {
    pub(in crate::application::file_dag) fn prepare_graph(
        &self,
        contract: &OperatorContract,
        admission: &InvocationWorkspace,
        mode: EvaluationMode,
        program: &ResourceProgram,
        rows: &[Rows<'_>],
    ) -> Result<Option<CachedGraph>, OperatingError> {
        if !matches!(program, ResourceProgram::Axis(_))
            && !matches!(program, ResourceProgram::Join(join) if join.reach.is_some())
        {
            return Ok(None);
        }
        if !std::ptr::eq(self, &admission.owner().derived) {
            return Err(OperatingError::NotRead);
        }
        let mut backend = self.backend.borrow_mut();
        let Some(cache) = backend.as_mut().filter(|cache| cache.graph_enabled) else {
            return Ok(None);
        };
        CachedGraph::prepare(contract, admission, mode, program, rows, Some(cache))
    }
    /// Only admitted original inputs enter the physical graph adapter.
    pub(in crate::application::file_dag) fn acquire_graph(
        &self,
        plan: &Arc<DerivedContract>,
        input: &crate::application::file_dag::workspace::GraphInput<'_>,
        cache: Option<&mut CacheBackend>,
    ) -> Result<Option<super::super::graph_cache::types::handles::GraphHandle>, OperatingError>
    {
        if !std::ptr::eq(self, input.owner()) {
            return Err(OperatingError::NotRead);
        }
        super::super::graph::acquire(plan, input, cache)
    }
    pub(in crate::application::file_dag) fn acquire_graph_domain(
        &self,
        plan: &Arc<DerivedContract>,
        graph: &super::super::graph_cache::types::handles::GraphHandle,
        input: &crate::application::file_dag::workspace::GraphInput<'_>,
        cache: Option<&mut CacheBackend>,
    ) -> Result<super::super::graph_cache::types::handles::DomainHandle, OperatingError> {
        if !std::ptr::eq(self, input.owner()) {
            return Err(OperatingError::NotRead);
        }
        super::super::graph::domain(plan, graph, input, cache)
    }
    pub(in crate::application::file_dag) fn graph_row<'a>(
        &self,
        graph: &'a mut CachedGraph,
        row: &[Value],
    ) -> Result<CachedGraphRow<'a>, OperatingError> {
        graph.row(row, self, self.backend.borrow_mut().as_mut())
    }
}

impl DerivedOwner {
    pub(in crate::application::file_dag::derived) fn graph_backings(
        &self,
        storage: &Arc<DerivedStorage>,
    ) -> Result<Arc<[super::super::ledger::Backing]>, OperatingError> {
        super::backings::view(storage)
    }
    /// Compatibility is an owner lookup guard, never HashMap key equality.
    /// Only the matching stable hash bucket is visited; expired matching views
    /// are unlinked before retry, without retaining position payloads in keys.
    pub(in crate::application::file_dag::derived) fn canonical_graph_requirement(
        &self,
        requested: GraphViewIdentity,
        populations: &[PopulationId],
    ) -> RequirementIdentity {
        use std::hash::BuildHasher;
        let requested = RequirementIdentity::Graph(Arc::new(requested));
        let mut ids = [PopulationId(usize::MAX); 2];
        ids[..populations.len()].copy_from_slice(populations);
        let probe = Key {
            plan: requested.clone(),
            populations: ids,
            len: populations.len() as u8,
            context: ViewContext::default(),
        };
        let mut state = self.state.borrow_mut();
        let hash = state.entries.hasher().hash_one(&probe);
        loop {
            // Inspect only this stable hash bucket. This is not map equality:
            // expired storage is removed before position compatibility runs.
            let entries = &state.entries;
            let expired = entries
                .raw_entry()
                .from_hash(hash, |key| {
                    key.populations == probe.populations
                        && key.len == probe.len
                        && key.context == probe.context
                        && key.plan.same_header(&probe.plan)
                        && entries
                            .get(key)
                            .is_some_and(|entry| entry.weak.strong_count() == 0)
                })
                .map(|(key, _)| key.clone());
            if let Some(key) = expired {
                state.entries.remove(&key);
                for population in &key.populations[..usize::from(key.len)] {
                    if let Some(keys) = state.dependents.get_mut(population) {
                        keys.retain(|known| known != &key);
                    }
                }
                continue;
            }
            let found = state
                .entries
                .raw_entry()
                .from_hash(hash, |key| {
                    key.populations == probe.populations
                        && key.len == probe.len
                        && key.context == probe.context
                        && key.plan.compatible(&probe.plan)
                })
                .map(|(key, entry)| (key.clone(), entry.weak.strong_count() != 0));
            let Some((key, live)) = found else {
                return requested;
            };
            if live {
                return key.plan;
            }
            state.entries.remove(&key);
            for population in &key.populations[..usize::from(key.len)] {
                if let Some(keys) = state.dependents.get_mut(population) {
                    keys.retain(|known| known != &key);
                }
            }
        }
    }
    /// Physical payload increments come from the one ledger; vector metadata
    /// remains a bounded capacity charge, never a scan of retained populations.
    pub(in crate::application::file_dag::derived) fn graph_retention_cost(
        &self,
        backings: &[super::super::ledger::Backing],
        metadata: usize,
    ) -> Result<usize, OperatingError> {
        self.ledger
            .borrow()
            .incremental_optional_bytes(backings)?
            .checked_add(metadata)
            .ok_or(OperatingError::ShapeMismatch)
    }
    pub(in crate::application::file_dag::derived) fn graph_room(
        &self,
        cache: &CacheBackend,
        cost: usize,
        memo: bool,
    ) -> bool {
        cache
            .bytes
            .checked_add(cost)
            .is_some_and(|n| n <= cache.limit)
            && (!memo
                || cache
                    .memo_bytes
                    .checked_add(cost)
                    .is_some_and(|n| n <= super::super::graph_cache::service::MEMO_LIMIT))
    }
    /// Graph, domain and successful memo admissions use the same authority.
    pub(in crate::application::file_dag::derived) fn try_graph_lease(
        &self,
        cache: &CacheBackend,
        backings: &[super::super::ledger::Backing],
        cost: usize,
        memo: bool,
    ) -> Result<Option<Arc<super::super::ledger::ResourceLease>>, OperatingError> {
        if !self.graph_room(cache, cost, memo) {
            return Ok(None);
        }
        let ledger = self.ledger.borrow();
        let extra = ledger.incremental_optional_bytes(backings)?;
        let units = ledger.incremental_optional_units(backings)?;
        let snapshot = ledger.snapshot();
        let fits = snapshot
            .view_optional_bytes
            .checked_add(extra)
            .is_some_and(|n| n <= cache.limit)
            && snapshot
                .optional_units
                .checked_add(units)
                .is_some_and(|n| n <= super::super::selection::MAX_RETAINED_POSITION_CAPACITY);
        drop(ledger);
        if fits {
            self.optional_lease(backings).map(Some)
        } else {
            Ok(None)
        }
    }
}
