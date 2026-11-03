//! Private application backend storage. Only derived-owner algorithms can access mutable fields.

use ryo_derived::domain::contract::types::KeyIndex;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

/// One exact immutable stored population and its ordered typed key schema.
#[derive(Debug)]
pub(in crate::application::file_dag::derived) struct Entry {
    /// Logical relation, never a pointer identity.
    pub relation: Relation,
    /// Key positions in argument order, including repeats.
    pub columns: Vec<ColumnIndex>,
    /// Static schema compared by contents.
    pub kinds: &'static [CellKind],
    /// Original row population of the frozen relation.
    pub rows: usize,
    /// Immutable index; callers own a handle before releasing the mutable cache borrow.
    pub index: Arc<KeyIndex>,
    pub payload_lease: Option<Arc<super::ledger::ResourceLease>>,
}

/// A file-local owner: dropping it releases all admitted allocations without eviction.
#[derive(Debug)]
pub(in crate::application::file_dag) struct CacheBackend {
    #[cfg(test)]
    pub(super) selector_descriptor: Option<usize>,
    /// Per-file active repetition counts indexed by request-prepared predicate group.
    pub(super) selector_repetition: Vec<u8>,
    pub(super) selector_repetition_failed: bool,
    /// Total reserved row-position slots actually retained in selections/fanouts.
    pub(super) retained_positions: usize,
    /// Immutable request-owned selector requirements, never file-owned payload.
    pub(super) selection_views:
        Option<Arc<crate::domain::stored_selection::types::PreparedSelectionViews>>,
    /// An active eligible graph consumer may request bounded adjacency admission.
    pub(super) graph_enabled: bool,
    /// Charged graph/domain/memo bytes within `bytes`.
    pub(super) graph_bytes: usize,
    /// Successful root and nearest memo payload admitted within its smaller sub-budget.
    pub(super) memo_bytes: usize,
    /// Complete immutable graph populations.
    pub(super) graphs: Vec<super::graph_cache::types::retained::GraphEntry>,
    /// Exact stable domain memberships.
    pub(super) graph_domains: Vec<super::graph_cache::types::retained::DomainEntry>,
    /// Mutable nearest answers remain owned by this generation.
    pub(super) nearest: Vec<super::graph_cache::types::retained::NearestMemo>,
    /// Aggregate requested/traversed work and graph admission accounting.
    pub(super) graph_counts: crate::domain::stored_graph::types::GraphCounts,
    /// Active pure stored descriptors; prepared metadata is owned by the worker's plans.
    pub(super) selectors: Vec<Arc<crate::domain::stored_selection::types::Selector>>,
    /// Successful exact selections admitted under this same byte allowance.
    pub(super) selections: Vec<crate::domain::stored_selection::types::Matches>,
    /// Bounded text-to-row fanouts owned by this complete physical file generation.
    pub(super) fanouts: Vec<crate::domain::stored_selection::types::LiteralFanout>,
    /// Candidate indexes are built only for columns shared by distinct active predicates.
    pub(super) discriminator_columns: Vec<crate::domain::stored_selection::types::CandidateColumn>,
    /// Aggregate compact-selection execution counters.
    pub(super) selection_counts: crate::domain::stored_selection::types::SelectionCounts,
    /// Explicit diagnostic runs only; absent and allocation-free in normal execution.
    pub(super) selection_histogram:
        Option<Vec<crate::domain::stored_selection::types::SelectorHistogram>>,
    /// Active relations whose physical read reported no partiality, of any missing kind.
    pub(super) eligible: Vec<Relation>,
    /// Admitted indexes and their exact keys; linear lookup is bounded by the byte budget.
    pub(super) entries: Vec<Entry>,
    /// Accounted actual capacities and conservative allocation metadata.
    pub(super) bytes: usize,
    /// Maximum retained cache bytes for this file and worker.
    pub(super) limit: usize,
    /// Physical stored-key builds, including invocation-local admission refusals.
    pub(super) builds: u64,
    /// Successful immutable index reuse.
    pub(super) reuses: u64,
    /// Eligible requests whose index could not be retained within the bound.
    pub(super) refusals: u64,
}

/// The index prepared for one join; no mutable cache borrow escapes acquisition.
#[derive(Debug)]
pub(in crate::application::file_dag) struct Acquired {
    /// A shared index, or the already-built index in the caller's local scratch.
    pub shared: Option<Arc<KeyIndex>>,
    pub slot: Option<usize>,
    /// Whether this acquisition physically built an index.
    pub built: bool,
    /// Test observation of the admission decision; production policy updates counters directly.
    #[cfg(test)]
    pub eligible: bool,
}

impl CacheBackend {
    pub(in crate::application::file_dag) fn observe_graph_walk(
        &mut self,
        expanded: u64,
        examined: u64,
    ) {
        self.graph_counts.expanded += expanded;
        self.graph_counts.examined += examined;
    }
}
