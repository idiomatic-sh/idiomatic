//! Private generation-local cache data, with no retained source or provider references.
use crate::domain::table_model::types::KeyIndex;
use ryo_language::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

/// One exact immutable stored population and its ordered typed key schema.
#[derive(Debug)]
pub(crate) struct Entry {
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
}

/// A file-local owner: dropping it releases all admitted allocations without eviction.
#[derive(Debug)]
pub(crate) struct FileIndexes {
    /// An active eligible graph consumer may request bounded adjacency admission.
    pub graph_enabled: bool,
    /// Charged graph/domain/memo bytes within `bytes`.
    pub graph_bytes: usize,
    /// Successful root and nearest memo payload admitted within its smaller sub-budget.
    pub memo_bytes: usize,
    /// Complete immutable graph populations.
    pub graphs: Vec<crate::domain::stored_graph::types::GraphEntry>,
    /// Exact stable domain memberships.
    pub graph_domains: Vec<crate::domain::stored_graph::types::DomainEntry>,
    /// Mutable nearest answers remain owned by this generation.
    pub nearest: Vec<crate::domain::stored_graph::types::NearestMemo>,
    /// Aggregate requested/traversed work and graph admission accounting.
    pub graph_counts: crate::domain::stored_graph::types::GraphCounts,
    /// Active pure stored descriptors; prepared metadata is owned by the worker's plans.
    pub selectors: Vec<Arc<crate::domain::stored_selection::types::Selector>>,
    /// Successful exact selections admitted under this same byte allowance.
    pub selections: Vec<crate::domain::stored_selection::types::Matches>,
    /// Candidate indexes are built only for columns shared by distinct active predicates.
    pub discriminator_columns: Vec<crate::domain::stored_selection::types::CandidateColumn>,
    /// Aggregate compact-selection execution counters.
    pub selection_counts: crate::domain::stored_selection::types::SelectionCounts,
    /// Explicit diagnostic runs only; absent and allocation-free in normal execution.
    pub selection_histogram: Option<Vec<crate::domain::stored_selection::types::SelectorHistogram>>,
    /// Active relations whose physical read reported no partiality, of any missing kind.
    pub eligible: Vec<Relation>,
    /// Admitted indexes and their exact keys; linear lookup is bounded by the byte budget.
    pub entries: Vec<Entry>,
    /// Accounted actual capacities and conservative allocation metadata.
    pub bytes: usize,
    /// Maximum retained cache bytes for this file and worker.
    pub limit: usize,
    /// Physical stored-key builds, including invocation-local admission refusals.
    pub builds: u64,
    /// Successful immutable index reuse.
    pub reuses: u64,
    /// Eligible requests whose index could not be retained within the bound.
    pub refusals: u64,
}

/// The index prepared for one join; no mutable cache borrow escapes acquisition.
#[derive(Debug)]
pub(crate) struct Acquired {
    /// A shared index, or the already-built index in the caller's local scratch.
    pub shared: Option<Arc<KeyIndex>>,
    /// Whether this acquisition physically built an index.
    pub built: bool,
    /// Whether the physical relation was eligible for sharing.
    pub eligible: bool,
}
