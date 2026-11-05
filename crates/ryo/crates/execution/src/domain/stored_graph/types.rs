//! Plain compact graph, population, traversal and bounded memo data.

use crate::domain::table_model::types::SelectionIdentity;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

pub(crate) mod traversal;

/// Exact immutable relation population within one file-generation owner.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Population {
    /// Provider relation; native domain populations use their distinct native relation.
    pub relation: Relation,
    /// Physical provider column types.
    pub kinds: &'static [CellKind],
    /// Physical provider population before selection.
    pub rows: usize,
    /// Native population identity, distinct from any stored semantic population.
    pub native_file: Option<FileId>,
    /// Exact ordered predicates/bindings, absent for the complete raw relation.
    pub selected: Option<Arc<SelectionIdentity>>,
    /// Original positions, including order/multiplicity, for a selected population.
    pub positions: Option<Arc<Vec<u32>>>,
}

/// The physical columns and population defining one adjacency, independent of lookup orientation.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphIdentity {
    /// Exact physical population and optional selection.
    pub population: Population,
    /// Canonically ordered physical endpoint columns.
    pub endpoints: [ColumnIndex; 2],
    /// Physical partition columns in the original argument order, including repeats.
    pub keys: Vec<ColumnIndex>,
}

/// Neighbor direction in the graph's canonical endpoint order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Orientation {
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
pub(crate) struct GraphSource {
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
pub(crate) struct DomainIdentity {
    /// Domain population, distinct from the graph's population.
    pub population: Population,
    /// Physical entity column.
    pub column: ColumnIndex,
}

/// Membership is independent of duplicate domain rows.
#[derive(Debug)]
pub(crate) struct GraphDomain {
    /// Exact domain identity when shared, absent for invocation-local computed rows.
    pub identity: Option<DomainIdentity>,
    /// Membership by dense vertex ID.
    pub members: Arc<Vec<bool>>,
}

/// One domain-specific nearest context; mutable answers remain under the generation owner.

/// Reusable invocation traversal memory; marks use epochs rather than clearing all vertices per root.
#[derive(Debug, Default)]
pub(crate) struct GraphWalk {
    /// Visitation epoch by dense vertex.
    pub marks: Vec<u32>,
    /// Nonzero current epoch; wrapping resets marks once.
    pub epoch: u32,
    /// Iterative BFS frontier.
    pub queue: std::collections::VecDeque<u32>,
    /// Successful vertices in encounter order, optionally sorted by the caller.
    pub reached: Vec<u32>,
    /// Iterative nearest forest path being compressed.
    pub path: Vec<u32>,
    /// Invocation-local nearest answers when stable memo admission is unavailable.
    pub answers: Vec<u32>,
    /// Physical vertices expanded in this invocation.
    pub expanded: u64,
    /// Physical outgoing edges examined in this invocation.
    pub examined: u64,
}

/// Opt-in-compatible aggregate work counts for one generation.
#[derive(Debug, Default)]
pub(crate) struct GraphCounts {
    /// Dormant ordinary indexes reclaimed only to admit a complete new adjacency.
    pub reclaimed_indexes: u64,
    /// Dormant successful selection payloads reclaimed without dropping live row handles.
    pub reclaimed_selections: u64,
    /// Actual payload bytes released, excluding retained metadata capacity.
    pub reclaimed_bytes: u64,
    /// Admissions blocked while ordinary payload is pinned by another consumer.
    pub pinned_refusals: u64,
    /// Highest accounted retained cache bytes, not process RSS or transient allocations.
    pub peak_bytes: usize,
    /// Complete physical adjacency builds.
    pub builds: u64,
    /// Compatible generation-local adjacency hits.
    pub reuses: u64,
    /// Complete local graphs whose retention was refused.
    pub refusals: u64,
    /// Actual source-key requests, retaining left-row multiplicity.
    pub roots: u64,
    /// Root requests that performed traversal.
    pub traversals: u64,
    /// Successful reached-ID memo hits.
    pub memo_hits: u64,
    /// Vertices whose outgoing adjacency was expanded.
    pub expanded: u64,
    /// Edges visited by demanded traversal.
    pub examined: u64,
    /// Nearest queries stopped at their first qualifying encounter.
    pub early_stops: u64,
    /// Forest answers reused without a parent-chain walk.
    pub forest_hits: u64,
}
