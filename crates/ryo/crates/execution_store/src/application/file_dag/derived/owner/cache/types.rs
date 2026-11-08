//! Private application backend storage. Only derived-owner algorithms can access mutable fields.

use crate::application::file_dag::derived::ledger::types::ByteCount;
use crate::application::file_dag::derived::owner::index::types::ByteLimit;
use crate::application::file_dag::derived::owner::types::{BuildCount, RefusalCount, ReuseCount};
use ryo_execution_domain::domain::stored_graph::types::{ExaminedVertices, ExpandedVertices};
use ryo_relations::domain::relations::types::Relation;

/// Whether an eligible graph consumer may request bounded adjacency admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphAdmission {
    Disabled,
    Enabled,
}

/// One owner bounds the shared position allocations it retains optionally.
pub(crate) const MAX_RETAINED_POSITION_CAPACITY: usize =
    512 * 1024;

/// A file-local owner: dropping it releases all admitted allocations without eviction.
#[derive(Debug)]
pub(crate) struct CacheBackend {
    /// An active eligible graph consumer may request bounded adjacency admission.
    pub(in crate::application::file_dag::derived::owner) graph_enabled: GraphAdmission,
    /// Charged graph/domain/memo bytes within `bytes`.
    pub(in crate::application::file_dag::derived::owner) graph_bytes: ByteCount,
    /// Successful root and nearest memo payload admitted within its smaller sub-budget.
    pub(in crate::application::file_dag::derived::owner) memo_bytes: ByteCount,
    /// Complete immutable graph populations.
    pub(in crate::application::file_dag::derived::owner) graphs: Vec<
        crate::application::file_dag::derived::owner::graphs::types::retained::types::GraphEntry,
    >,
    /// Exact stable domain memberships.
    pub(in crate::application::file_dag::derived::owner) graph_domains: Vec<
        crate::application::file_dag::derived::owner::graphs::types::retained::types::DomainEntry,
    >,
    /// Mutable nearest answers remain owned by this generation.
    pub(in crate::application::file_dag::derived::owner) nearest: Vec<
        crate::application::file_dag::derived::owner::graphs::types::retained::types::NearestMemo,
    >,
    /// Aggregate requested/traversed work and graph admission accounting.
    pub(in crate::application::file_dag::derived::owner) graph_counts:
        ryo_execution_domain::domain::stored_graph::types::GraphCounts,
    /// Active relations whose physical read reported no partiality, of any missing kind.
    pub(in crate::application::file_dag::derived::owner) eligible: Vec<Relation>,
    /// Accounted actual capacities and conservative allocation metadata.
    pub(in crate::application::file_dag::derived::owner) bytes: ByteCount,
    /// Maximum retained cache bytes for this file and worker.
    pub(in crate::application::file_dag::derived::owner) limit: ByteLimit,
    /// Physical stored-key builds, including invocation-local admission refusals.
    pub(in crate::application::file_dag::derived::owner) builds: BuildCount,
    /// Successful immutable index reuse.
    pub(in crate::application::file_dag::derived::owner) reuses: ReuseCount,
    /// Eligible requests whose index could not be retained within the bound.
    pub(in crate::application::file_dag::derived::owner) refusals: RefusalCount,
}

impl CacheBackend {
    pub(crate) fn observe_graph_walk(
        &mut self,
        expanded: ExpandedVertices,
        examined: ExaminedVertices,
    ) {
        self.graph_counts.expanded.0 += expanded.0;
        self.graph_counts.examined.0 += examined.0;
    }
}
