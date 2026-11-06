//! Private application backend storage. Only derived-owner algorithms can access mutable fields.

use crate::application::file_dag::derived::owner::types::{BuildCount, RefusalCount, ReuseCount};
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

/// The address of the installed request descriptors, kept to prove a descriptor is not rebuilt.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DescriptorAddress(pub usize);

/// A request-prepared predicate group whose repetitions are counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RepetitionGroup(pub usize);

/// How many active selectors of one group repeat the same descriptor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Repetitions(pub u8);

/// Whether an eligible graph consumer may request bounded adjacency admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphAdmission {
    Disabled,
    Enabled,
}

/// Whether repetition counts could be recorded for every active group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepetitionTracking {
    Tracked,
    Failed,
}

/// Graph vertices expanded by one walk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExpandedVertices(pub u64);

/// Graph vertices examined by one walk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExaminedVertices(pub u64);

/// A file-local owner: dropping it releases all admitted allocations without eviction.
#[derive(Debug)]
pub(in crate::application::file_dag) struct CacheBackend {
    #[cfg(test)]
    pub(in crate::application::file_dag::derived::owner) selector_descriptor:
        Option<DescriptorAddress>,
    /// Sparse active repetition counts keyed by request-prepared predicate group.
    pub(in crate::application::file_dag::derived::owner) selector_repetition:
        std::collections::HashMap<RepetitionGroup, Repetitions>,
    pub(in crate::application::file_dag::derived::owner) selector_repetition_failed:
        RepetitionTracking,
    /// Immutable request-owned selector requirements, never file-owned payload.
    pub(in crate::application::file_dag::derived::owner) selection_views:
        Option<Arc<crate::domain::stored_selection::types::PreparedSelectionViews>>,
    /// An active eligible graph consumer may request bounded adjacency admission.
    pub(in crate::application::file_dag::derived::owner) graph_enabled: GraphAdmission,
    /// Charged graph/domain/memo bytes within `bytes`.
    pub(in crate::application::file_dag::derived::owner) graph_bytes: usize,
    /// Successful root and nearest memo payload admitted within its smaller sub-budget.
    pub(in crate::application::file_dag::derived::owner) memo_bytes: usize,
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
        crate::domain::stored_graph::types::GraphCounts,
    /// Active pure stored descriptors; prepared metadata is owned by the worker's plans.
    pub(in crate::application::file_dag::derived::owner) selectors:
        Vec<Arc<crate::domain::stored_selection::types::Selector>>,
    /// Active immutable literal inputs prepared once for this generation; contains no row payload.
    pub(in crate::application::file_dag::derived::owner) active_postings: Vec<ActivePostings>,
    /// Candidate indexes are built only for columns shared by distinct active predicates.
    pub(in crate::application::file_dag::derived::owner) discriminator_columns:
        Vec<crate::domain::stored_selection::types::CandidateColumn>,
    /// Aggregate compact-selection execution counters.
    pub(in crate::application::file_dag::derived::owner) selection_counts:
        crate::domain::stored_selection::types::SelectionCounts,
    /// Explicit diagnostic runs only; absent and allocation-free in normal execution.
    pub(in crate::application::file_dag::derived::owner) selection_histogram:
        Option<Vec<crate::domain::stored_selection::types::SelectorHistogram>>,
    /// Active relations whose physical read reported no partiality, of any missing kind.
    pub(in crate::application::file_dag::derived::owner) eligible: Vec<Relation>,
    /// Accounted actual capacities and conservative allocation metadata.
    pub(in crate::application::file_dag::derived::owner) bytes: usize,
    /// Maximum retained cache bytes for this file and worker.
    pub(in crate::application::file_dag::derived::owner) limit: usize,
    /// Physical stored-key builds, including invocation-local admission refusals.
    pub(in crate::application::file_dag::derived::owner) builds: BuildCount,
    /// Successful immutable index reuse.
    pub(in crate::application::file_dag::derived::owner) reuses: ReuseCount,
    /// Eligible requests whose index could not be retained within the bound.
    pub(in crate::application::file_dag::derived::owner) refusals: RefusalCount,
}

#[derive(Debug)]
pub(in crate::application::file_dag::derived::owner) struct ActivePostings {
    pub relation: Relation,
    pub column: ryo_values::domain::plan_parts::types::ColumnIndex,
    pub literals: Arc<[ryo_values::domain::plan_parts::types::LiteralText]>,
}

impl CacheBackend {
    /// Install immutable request descriptors without exposing their catalog to a caller.
    pub(in crate::application::file_dag::derived::owner) fn bind_request_views(
        &mut self,
        views: Arc<crate::domain::stored_selection::types::PreparedSelectionViews>,
    ) {
        self.selection_views = Some(views);
    }
    pub(in crate::application::file_dag::derived::owner) fn observe_graph_walk(
        &mut self,
        expanded: ExpandedVertices,
        examined: ExaminedVertices,
    ) {
        self.graph_counts.expanded += expanded.0;
        self.graph_counts.examined += examined.0;
    }
}
