//! Plain compact graph, population, traversal and bounded memo data.
use ryo_derived::domain::vertices::types::VertexId;

use ryo_relations::domain::relations::types::{CellKind, FileId, Relation, Value};
use ryo_derived::domain::sharing::types::SharedMarks;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowCount;

pub mod traversal;

/// Exact immutable relation population within one file-generation owner.
#[derive(Clone, Debug, PartialEq)]
pub struct RelationPopulation {
    /// Provider relation; native domain populations use their distinct native relation.
    pub relation: Relation,
    /// Physical provider column types.
    pub kinds: &'static [CellKind],
    /// Physical provider population.
    pub rows: RowCount,
    /// Native population identity, distinct from any stored semantic population.
    pub native_file: Option<FileId>,
}

/// The physical columns and population defining one adjacency, independent of lookup orientation.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphIdentity {
    /// Exact physical population and optional selection.
    pub population: RelationPopulation,
    /// Canonically ordered physical endpoint columns.
    pub endpoints: [ColumnIndex; 2],
    /// Physical partition columns in the original argument order, including repeats.
    pub keys: Vec<ColumnIndex>,
}

/// Neighbor direction in the graph's canonical endpoint order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    /// First endpoint to second.
    Forward,
    /// Second endpoint to first.
    Reverse,
}

/// Complete owned adjacency: no borrowed provider rows, source bytes or symbol buffers.

/// One successful sorted positive-reach answer, bounded independently of required live traversal.

/// Admitted adjacency with bounded successful root memoization.

/// A graph handle and requested orientation; no mutable cache borrow survives acquisition.

/// Canonical graph identity plus the columns needed to build it from the current logical row view.
pub struct GraphSource {
    /// Exact physical population key.
    pub identity: GraphIdentity,
    /// Canonical endpoint columns in the current row view.
    pub endpoints: [ColumnIndex; 2],
    /// Requested source orientation.
    pub orientation: Orientation,
}

/// Shared domain membership belongs to one exact adjacency as well as its own population.

/// Exact domain membership context for graph/nearest reuse.
#[derive(Clone, Debug, PartialEq)]
pub struct DomainIdentity {
    /// Domain population, distinct from the graph's population.
    pub population: RelationPopulation,
    /// Physical entity column.
    pub column: ColumnIndex,
}

/// Membership is independent of duplicate domain rows.
#[derive(Debug)]
pub struct GraphDomain {
    /// Exact domain identity when shared, absent for invocation-local computed rows.
    pub identity: Option<DomainIdentity>,
    /// Membership by dense vertex ID.
    pub members: SharedMarks,
}

/// One domain-specific nearest context; mutable answers remain under the generation owner.

/// The dense vertices one vertex reaches in one orientation, in the adjacency's own numbering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Neighbors<'a>(pub &'a [u32]);

/// The traversal in which a vertex was last visited; zero marks no traversal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisitEpoch(pub u32);

/// Graph vertices expanded by one walk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExpandedVertices(pub u64);

/// Graph edges examined by one walk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExaminedVertices(pub u64);

/// Reusable invocation traversal memory; marks use epochs rather than clearing all vertices per root.
#[derive(Debug, Default)]
pub struct GraphWalk {
    /// Visitation epoch by dense vertex.
    pub marks: Vec<VisitEpoch>,
    /// Nonzero current epoch; wrapping resets marks once.
    pub epoch: VisitEpoch,
    /// Iterative BFS frontier.
    pub queue: std::collections::VecDeque<VertexId>,
    /// Successful vertices in encounter order, optionally sorted by the caller.
    pub reached: Vec<VertexId>,
    /// Iterative nearest forest path being compressed.
    pub path: Vec<VertexId>,
    /// Invocation-local nearest answers when stable memo admission is unavailable.
    pub answers: Vec<VertexId>,
    /// Physical vertices expanded in this invocation.
    pub expanded: ExpandedVertices,
    /// Physical outgoing edges examined in this invocation.
    pub examined: ExaminedVertices,
}

/// Whether a breadth traversal ran to its end or stopped at its first qualifying encounter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreadthEnd {
    /// Every reachable vertex was visited.
    Complete,
    /// The traversal stopped at its first qualifying vertex.
    Stopped,
}

/// Whether a nearest answer came from an earlier walk of the validated forest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoStatus {
    /// The answer was computed by this call.
    Fresh,
    /// The answer was already known.
    Memoized,
}

/// The nearest qualifying ancestor of a vertex and where the answer came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nearest {
    /// The ancestor, absent when none qualifies or the orientation has no validated forest.
    pub vertex: Option<VertexId>,
    /// Whether the answer was already known.
    pub memo: MemoStatus,
}

/// Dormant ordinary indexes reclaimed only to admit a complete new adjacency.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReclaimedIndexes(pub u64);

/// Actual payload bytes released, excluding retained metadata capacity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReclaimedBytes(pub u64);

/// Admissions blocked while ordinary payload is pinned by another consumer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PinnedRefusals(pub u64);

/// Highest accounted retained cache bytes, not process RSS or transient allocations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PeakBytes(pub usize);

/// Complete physical adjacency builds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphBuilds(pub u64);

/// Compatible generation-local adjacency hits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphReuses(pub u64);

/// Complete local graphs whose retention was refused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphRefusals(pub u64);

/// Actual source-key requests, retaining left-row multiplicity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RootRequests(pub u64);

/// Root requests that performed traversal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Traversals(pub u64);

/// Successful reached-ID memo hits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoHits(pub u64);

/// Nearest queries stopped at their first qualifying encounter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EarlyStops(pub u64);

/// Forest answers reused without a parent-chain walk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForestHits(pub u64);

/// Opt-in-compatible aggregate work counts for one generation.
#[derive(Debug, Default)]
pub struct GraphCounts {
    /// Dormant ordinary indexes reclaimed only to admit a complete new adjacency.
    pub reclaimed_indexes: ReclaimedIndexes,
    /// Actual payload bytes released, excluding retained metadata capacity.
    pub reclaimed_bytes: ReclaimedBytes,
    /// Admissions blocked while ordinary payload is pinned by another consumer.
    pub pinned_refusals: PinnedRefusals,
    /// Highest accounted retained cache bytes, not process RSS or transient allocations.
    pub peak_bytes: PeakBytes,
    /// Complete physical adjacency builds.
    pub builds: GraphBuilds,
    /// Compatible generation-local adjacency hits.
    pub reuses: GraphReuses,
    /// Complete local graphs whose retention was refused.
    pub refusals: GraphRefusals,
    /// Actual source-key requests, retaining left-row multiplicity.
    pub roots: RootRequests,
    /// Root requests that performed traversal.
    pub traversals: Traversals,
    /// Successful reached-ID memo hits.
    pub memo_hits: MemoHits,
    /// Vertices whose outgoing adjacency was expanded.
    pub expanded: ExpandedVertices,
    /// Edges visited by demanded traversal.
    pub examined: ExaminedVertices,
    /// Nearest queries stopped at their first qualifying encounter.
    pub early_stops: EarlyStops,
    /// Forest answers reused without a parent-chain walk.
    pub forest_hits: ForestHits,
}
