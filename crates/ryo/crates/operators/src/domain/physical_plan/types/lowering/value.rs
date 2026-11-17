//! The `Lowering` value: the kernels one node of the physical plan is lowered to, and the rows they read. The plan's one list of
//! operators generates it from the nodes below; each operator declares its growth beside its kernel.
use crate::domain::aggregate::types::GroupParts;
use crate::domain::append::types::ExtendParts;
use crate::domain::axis::types::AxisParts;
use crate::domain::closure::types::ReachParts;
use crate::domain::cycles::types::CycleParts;
use crate::domain::equality_join::types::{ProbeParts, SeededJoin};
use crate::domain::ordered_dfs::types::DfsParts;
use crate::domain::ordered_match::types::OrderedParts;
use crate::domain::physical_plan::types::{IndexPlace, ProbeIndex, RowMap, RowSource};
use crate::domain::project::types::ProjectParts;
use crate::domain::render_batch::types::RenderParts;
use crate::domain::select::types::SelectParts;
use crate::domain::worklist::types::WorklistParts;
use ryo_relations::domain::relations::types::TextHash;
use ryo_values::domain::plan_parts::types::{HostedNumber, ParameterId, TableKey};
use ryo_values::domain::query_bindings::types::ParameterBinding;

crate::lowerings! {
    /// `filter`: the rows of the source the comparisons hold for, with the kept columns.
    Filter(FilterNode),
    /// `lookup` of a constant in a relation's shared `hash_index`, then `filter` of all the select's comparisons.
    Lookup(LookupNode),
    /// `project`: the columns in order, repeated rows dropped when asked.
    Project(ProjectNode),
    /// `extend`: each row followed by the values, resolved once per file.
    Extend(ExtendNode),
    /// `hash_index` of the right rows, then `probe` of it with each left row.
    Probe(ProbeNode),
    /// `distinct` over the rows of each source in turn: a row the first time only.
    Distinct(DistinctNode),
    /// `group`: the rows keyed through `hash_index`'s table, one row per group in the order of its first row, with its count.
    Group(GroupNode),
    /// `reach` from every source (`traverse::All`): one row per partition, source and entity it reaches.
    Reach(ReachNode),
    /// `reach` from the sources the left rows bind (`traverse::Seeds`), probed by them.
    SeededReach(SeededReachNode),
    /// `directed_walk`: from each start row, the entities a walk over the edges reaches in a direction the domain admits.
    Axis(AxisNode),
    /// `ordered_walk`: from each distinct partition and root, in sorted order, the edges an ordered depth-first visit reaches.
    OrderedWalk(OrderedWalkNode),
    /// `worklist`: from each distinct partition and root, in sorted order, the edges a first-in-first-out dataflow follows.
    Worklist(WorklistNode),
    /// `components`: each entity of a strongly connected component of more than one, or with an edge to itself, once.
    Components(ComponentsNode),
    /// `text_map`: each row of the source through one text kind's per-row meaning, giving none, one or many rows.
    TextMap(TextMapNode),
    /// `band_join`: each left row with the right rows of its key inside a band of order comparisons, every one or the nearest.
    BandJoin(BandJoinNode),
    /// `ordered_match`: both sources ordered by key then order, each left row paired with the earliest later right row.
    OrderedMatch(OrderedMatchNode),
    /// `render`: one batch of renderings for every row of the source, refused whole on one unsupported answer.
    Render(RenderNode),
    /// `scan_bound`: the rows the engine holds for the run, or a table parameter's rows.
    ScanBound(ScanBoundNode),
}

/// A `filter` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FilterNode {
    /// What it reads.
    pub(crate) source: RowSource,
    /// The comparisons, the kept columns and the bindings.
    pub(crate) select: SelectParts,
}

/// A `lookup` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LookupNode {
    /// The index it reads.
    pub(crate) index: IndexPlace,
    /// The hash of the constant the key equals.
    pub(crate) key: TextHash,
    /// The comparisons, the kept columns and the bindings.
    pub(crate) select: SelectParts,
}

/// A `project` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectNode {
    /// What it reads.
    pub(crate) source: RowSource,
    /// The columns and whether a repeated row is kept once.
    pub(crate) project: ProjectParts,
}

/// An `extend` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ExtendNode {
    /// What it reads.
    pub(crate) source: RowSource,
    /// The values and what their parameters are bound to.
    pub(crate) extension: ExtendParts,
}

/// A `probe` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProbeNode {
    /// The rows probed with, in their order.
    pub(crate) left: RowSource,
    /// The rows indexed.
    pub(crate) right: RowSource,
    /// What the probe keeps.
    pub(crate) probe: ProbeParts,
    /// The index it probes: its own, or the one every probe of its right rows and key shares.
    pub(crate) index: ProbeIndex,
}

/// A `distinct` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DistinctNode {
    /// What it reads, in order.
    pub(crate) sources: [RowSource; 2],
}

/// A `group` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupNode {
    /// What it reads.
    pub(crate) source: RowSource,
    /// The keys and what is counted.
    pub(crate) group: GroupParts,
}

/// A `reach` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReachNode {
    /// The edge rows.
    pub(crate) edges: RowSource,
    /// The graph they make.
    pub(crate) graph: ReachParts,
}

/// A seeded `reach` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SeededReachNode {
    /// The left rows, read in order.
    pub(crate) left: RowSource,
    /// The edge rows.
    pub(crate) edges: RowSource,
    /// The graph they make.
    pub(crate) graph: ReachParts,
    /// The join over the reached rows.
    pub(crate) join: SeededJoin,
}

/// A `directed_walk` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxisNode {
    /// The start rows, read in order.
    pub(crate) starts: RowSource,
    /// The edge rows.
    pub(crate) edges: RowSource,
    /// The domain rows.
    pub(crate) domain: RowSource,
    /// The walk.
    pub(crate) axis: AxisParts,
}

/// An `ordered_walk` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OrderedWalkNode {
    /// The root rows.
    pub(crate) roots: RowSource,
    /// The edge rows.
    pub(crate) edges: RowSource,
    /// The visit.
    pub(crate) dfs: DfsParts,
}

/// A `worklist` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorklistNode {
    /// The root rows.
    pub(crate) roots: RowSource,
    /// The edge rows.
    pub(crate) edges: RowSource,
    /// The lattice rows.
    pub(crate) lattice: RowSource,
    /// The worklist.
    pub(crate) worklist: WorklistParts,
}

/// A `components` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComponentsNode {
    /// The edge rows.
    pub(crate) edges: RowSource,
    /// The graph they make.
    pub(crate) cycles: CycleParts,
}

/// A `text_map` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextMapNode {
    /// What it reads.
    pub(crate) source: RowSource,
    /// The kind's parts.
    pub(crate) map: RowMap,
}

/// An `ordered_match` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OrderedMatchNode {
    /// The left rows.
    pub(crate) left: RowSource,
    /// The right rows.
    pub(crate) right: RowSource,
    /// The keys, the orders and the output.
    pub(crate) ordered: OrderedParts,
}

/// A `render` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderNode {
    /// What it reads.
    pub(crate) source: RowSource,
    /// The node column and the role.
    pub(crate) render: RenderParts,
}

/// A `scan_bound` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ScanBoundNode {
    /// What it scans.
    pub(crate) scan: BoundScan,
}

/// A `band_join` node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BandJoinNode {
    /// The left rows, read in order.
    pub(crate) left: RowSource,
    /// The right rows.
    pub(crate) right: RowSource,
    /// The keys, the band, the answer and the output.
    pub(crate) band: crate::domain::equality_join::types::BandParts,
}

/// What a bound scan reads: the host's rows of a declared relation, or the tables a `list<table>` parameter is bound to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BoundScan {
    /// The host's rows of a declared relation.
    Hosted(HostedNumber),
    /// One row for each table of a parameter, the text under each key in key order.
    Rows {
        /// The parameter.
        parameter: ParameterId,
        /// The keys, in order.
        keys: Vec<TableKey>,
        /// What the node's parameters are bound to.
        bindings: Vec<ParameterBinding>,
    },
}

#[cfg(test)]
kinds! {
    /// The kind of a `RowMap`, without its parts: what a test that must see every kind compares against.
    pub(crate) enum RowMapKind {
        /// `capture`.
        Capture,
        /// `replace`.
        Replace,
        /// `source-matches`.
        SourceMatches,
        /// `span-bounds`.
        SpanBounds,
        /// `span-between`.
        SpanBetween,
        /// `words`.
        Words,
        /// `rank`.
        Rank,
        /// `fits-rendered`.
        FitsRendered,
    }
}

impl Lowering {
    /// The rows the node reads, in the order its kernels read them; a lookup reads its index's relation, which is not here.
    pub(crate) fn sources(&self) -> impl Iterator<Item = RowSource> {
        let (first, second, third) = match self {
            Self::Filter(FilterNode { source, .. })
            | Self::Project(ProjectNode { source, .. })
            | Self::Extend(ExtendNode { source, .. })
            | Self::Group(GroupNode { source, .. })
            | Self::TextMap(TextMapNode { source, .. })
            | Self::Render(RenderNode { source, .. }) => (Some(*source), None, None),
            Self::Reach(ReachNode { edges, .. })
            | Self::Components(ComponentsNode { edges, .. }) => (Some(*edges), None, None),
            Self::Probe(ProbeNode { left, right, .. })
            | Self::BandJoin(BandJoinNode { left, right, .. })
            | Self::OrderedMatch(OrderedMatchNode { left, right, .. }) => {
                (Some(*left), Some(*right), None)
            }
            Self::Distinct(DistinctNode {
                sources: [left, right],
            }) => (Some(*left), Some(*right), None),
            Self::SeededReach(SeededReachNode { left, edges, .. }) => {
                (Some(*left), Some(*edges), None)
            }
            Self::OrderedWalk(OrderedWalkNode { roots, edges, .. }) => {
                (Some(*roots), Some(*edges), None)
            }
            Self::Axis(AxisNode {
                starts,
                edges,
                domain,
                ..
            }) => (Some(*starts), Some(*edges), Some(*domain)),
            Self::Worklist(WorklistNode {
                roots,
                edges,
                lattice,
                ..
            }) => (Some(*roots), Some(*edges), Some(*lattice)),
            Self::Lookup(_) | Self::ScanBound(_) => (None, None, None),
        };
        first.into_iter().chain(second).chain(third)
    }
}
