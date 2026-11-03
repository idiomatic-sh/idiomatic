//! Private application backend storage. Only derived-owner algorithms can access mutable fields.

use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

/// A file-local owner: dropping it releases all admitted allocations without eviction.
#[derive(Debug)]
pub(in crate::application::file_dag) struct CacheBackend {
    #[cfg(test)]
    pub(super) selector_descriptor: Option<usize>,
    /// Per-file active repetition counts indexed by request-prepared predicate group.
    pub(super) selector_repetition: Vec<u8>,
    pub(super) selector_repetition_failed: bool,
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
    /// Active immutable literal inputs prepared once for this generation; contains no row payload.
    pub(super) active_postings: Vec<ActivePostings>,
    /// Candidate indexes are built only for columns shared by distinct active predicates.
    pub(super) discriminator_columns: Vec<crate::domain::stored_selection::types::CandidateColumn>,
    /// Aggregate compact-selection execution counters.
    pub(super) selection_counts: crate::domain::stored_selection::types::SelectionCounts,
    /// Explicit diagnostic runs only; absent and allocation-free in normal execution.
    pub(super) selection_histogram:
        Option<Vec<crate::domain::stored_selection::types::SelectorHistogram>>,
    /// Active relations whose physical read reported no partiality, of any missing kind.
    pub(super) eligible: Vec<Relation>,
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

#[derive(Debug)]
pub(super) struct ActivePostings {
    pub relation: Relation,
    pub column: ryo_values::domain::plan_parts::types::ColumnIndex,
    pub literals: Arc<[ryo_values::domain::plan_parts::types::LiteralText]>,
}
