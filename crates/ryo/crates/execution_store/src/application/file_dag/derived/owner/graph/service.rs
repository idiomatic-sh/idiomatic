//! The `graph` application operation.
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::service::DerivedStorage;
use crate::application::file_dag::derived::owner::graph_key::service::GraphViewIdentity;
use crate::application::file_dag::derived::owner::graph_key::service::RequirementIdentity;
use crate::application::file_dag::derived::population::service::PopulationId;
use std::sync::Arc;

use crate::application::file_dag::accounting::ledger::types::ByteCount;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use ryo_execution_domain::domain::{
    derived_views::types::contract::value::DerivedContract, operating::errors::OperatingError,
};

use super::super::service::*;

use super::super::types::*;
use crate::application::file_dag::derived::owner::cache::types::GraphAdmission;
use crate::application::file_dag::{
    derived::owner::cached_graph::service::CachedGraph, workspace::service::InvocationWorkspace,
};
use ryo_execution_domain::domain::{
    operator::contract::{
        graph_view::types::CachedGraphRow, resources::types::ResourceProgram,
        types::OperatorContract,
    },
    table_model::types::Rows,
};
use ryo_relations::domain::relations::types::Value;

impl DerivedOwner {
    /// Prepares the cached graph an axis or reach operator reads, when its program caches one.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRead`] when `admission` belongs to another owner, [`OperatingError::NotRun`] when the file's
    /// graph cache is not prepared, and the refusals of acquiring the graph and its domain.
    pub(crate) fn prepare_graph(
        &self,
        contract: &OperatorContract,
        admission: &InvocationWorkspace,
        program: &ResourceProgram,
        rows: &[Rows<'_>],
    ) -> Result<Option<CachedGraph>, OperatingError> {
        if !matches!(program, ResourceProgram::Axis(_))
            && !matches!(program, ResourceProgram::Join(join) if join.reach.is_some())
        {
            return Ok(None);
        }
        if !std::ptr::eq(self, admission.owner().derived()) {
            return Err(OperatingError::NotRead);
        }
        // The project owner holds no graph cache: its axis or reach walks the frontier tables, as an uncached one does.
        if self.file_prepared.get() == FilePreparation::Project {
            return Ok(None);
        }
        let mut cache = self.graph_cache()?;
        if cache.graph_enabled != GraphAdmission::Enabled {
            return Ok(None);
        }
        CachedGraph::prepare(contract, admission, program, rows, &mut cache)
    }
    /// The one place a graph demand reaches the cache: before `prepare_program_file`, after
    /// retirement, or when the file's cache was refused, the demand is refused with
    /// `NotRun` and never traversed by an ordinary route. The project owner never reaches it:
    /// `prepare_graph` declines the graph there.
    pub(crate) fn graph_cache(
        &self,
    ) -> Result<std::cell::RefMut<'_, CacheBackend>, OperatingError> {
        std::cell::RefMut::filter_map(self.backend.borrow_mut(), Option::as_mut)
            .map_err(|_| OperatingError::NotRun)
    }
    /// Only admitted original inputs enter the physical graph adapter.
    pub(crate) fn acquire_graph(
        &self,
        plan: &Arc<DerivedContract>,
        input: &crate::application::file_dag::workspace::service::GraphInput<'_>,
        cache: &mut CacheBackend,
    ) -> Result<
        Option<crate::application::file_dag::derived::owner::graphs::handles::service::GraphHandle>,
        OperatingError,
    > {
        if !std::ptr::eq(self, input.owner()) {
            return Err(OperatingError::NotRead);
        }
        super::super::acquisition::graph::service::acquire(plan, input, cache)
    }
    pub(crate) fn acquire_graph_domain(
        &self,
        plan: &Arc<DerivedContract>,
        graph: &crate::application::file_dag::derived::owner::graphs::handles::service::GraphHandle,
        input: &crate::application::file_dag::workspace::service::GraphInput<'_>,
        cache: &mut CacheBackend,
    ) -> Result<
        crate::application::file_dag::derived::owner::graphs::handles::service::DomainHandle,
        OperatingError,
    > {
        if !std::ptr::eq(self, input.owner()) {
            return Err(OperatingError::NotRead);
        }
        super::super::acquisition::graph::service::domain(plan, graph, input, cache)
    }
    /// The cached graph's answer for one row.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] when the file's graph cache is not prepared, and the graph's refusal of the row.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedOwner;
    /// ```
    pub fn graph_row<'a>(
        &self,
        graph: &'a mut CachedGraph,
        row: &[Value],
    ) -> Result<CachedGraphRow<'a>, OperatingError> {
        let mut cache = self.graph_cache()?;
        graph.row(row, self, &mut cache)
    }
}

impl DerivedOwner {
    pub(crate) fn graph_backings(
        &self,
        storage: &Arc<DerivedStorage>,
    ) -> Result<
        Arc<[crate::application::file_dag::accounting::ledger::types::Backing]>,
        OperatingError,
    > {
        super::super::backings::service::view(storage)
    }
    /// IdentityCompatibility is an owner lookup guard, never HashMap key equality.
    /// Only the matching stable hash bucket is visited; expired matching views
    /// are unlinked before retry, without retaining position payloads in keys.
    pub(crate) fn canonical_graph_requirement(
        &self,
        requested: GraphViewIdentity,
        populations: &[PopulationId],
    ) -> Result<RequirementIdentity, OperatingError> {
        use std::hash::BuildHasher;
        let requested = RequirementIdentity::Graph(Arc::new(requested));
        let mut ids = [PopulationId(usize::MAX); 2];
        ids[..populations.len()].copy_from_slice(populations);
        let Ok(len) = u8::try_from(populations.len()) else {
            return Err(OperatingError::ShapeMismatch);
        };
        let probe = Key {
            plan: requested.clone(),
            populations: ids,
            len: PopulationCount(len),
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
                        && key.plan.same_header(&probe.plan) == IdentityCompatibility::Compatible
                        && entries
                            .get(key)
                            .is_some_and(|entry| entry.weak.strong_count() == 0)
                })
                .map(|(key, _)| key.clone());
            if let Some(key) = expired {
                state.entries.remove(&key);
                for population in &key.populations[..usize::from(key.len.0)] {
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
                        && key.plan.compatible(&probe.plan) == IdentityCompatibility::Compatible
                })
                .map(|(key, entry)| (key.clone(), entry.weak.strong_count() != 0));
            let Some((key, live)) = found else {
                return Ok(requested);
            };
            if live {
                return Ok(key.plan);
            }
            state.entries.remove(&key);
            for population in &key.populations[..usize::from(key.len.0)] {
                if let Some(keys) = state.dependents.get_mut(population) {
                    keys.retain(|known| known != &key);
                }
            }
        }
    }
    /// Physical payload increments come from the one ledger; vector metadata
    /// remains a bounded capacity charge, never a scan of retained populations.
    pub(crate) fn graph_retention_cost(
        &self,
        backings: &[crate::application::file_dag::accounting::ledger::types::Backing],
        metadata: ByteCount,
    ) -> Result<ByteCount, OperatingError> {
        self.ledger
            .borrow()
            .incremental_optional_bytes(backings)?
            .0
            .checked_add(metadata.0)
            .map(ByteCount)
            .ok_or(OperatingError::ShapeMismatch)
    }
    pub(crate) fn graph_room(
        &self,
        cache: &CacheBackend,
        cost: ByteCount,
        memo: MemoBudget,
    ) -> Room {
        Room::from(
            cache
                .bytes
                .0
                .checked_add(cost.0)
                .is_some_and(|n| n <= cache.limit.0)
                && (memo == MemoBudget::Exempt
                    || cache.memo_bytes.0.checked_add(cost.0).is_some_and(|n| {
                        n <= crate::application::file_dag::derived::owner::graphs::service::MEMO_LIMIT
                    })),
        )
    }
    /// Graph, domain and successful memo admissions use the same authority.
    pub(crate) fn try_graph_lease(
        &self,
        cache: &CacheBackend,
        backings: &[crate::application::file_dag::accounting::ledger::types::Backing],
        cost: ByteCount,
        memo: MemoBudget,
    ) -> Result<
        Option<Arc<crate::application::file_dag::accounting::ledger::service::ResourceLease>>,
        OperatingError,
    > {
        if self.graph_room(cache, cost, memo) == Room::Full {
            return Ok(None);
        }
        let ledger = self.ledger.borrow();
        let extra = ledger.incremental_optional_bytes(backings)?.0;
        let units = ledger.incremental_optional_units(backings)?.0;
        let snapshot = ledger.snapshot();
        let fits = snapshot
            .view_optional_bytes
            .0
            .checked_add(extra)
            .is_some_and(|n| n <= cache.limit.0)
            && snapshot
                .optional_units
                .0
                .checked_add(units)
                .is_some_and(|n| n <= crate::application::file_dag::derived::owner::cache::types::MAX_RETAINED_POSITION_CAPACITY);
        drop(ledger);
        if fits {
            self.optional_lease(backings).map(Some)
        } else {
            Ok(None)
        }
    }
}
