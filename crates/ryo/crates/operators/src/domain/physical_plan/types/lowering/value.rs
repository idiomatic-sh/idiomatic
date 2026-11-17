//! The `Lowering` value: the kernels one node of the physical plan is lowered to, and the rows they read.
use crate::domain::aggregate::types::GroupParts;
use crate::domain::append::types::ExtendParts;
use crate::domain::axis::types::AxisParts;
use crate::domain::closure::types::ReachParts;
use crate::domain::cycles::types::CycleParts;
use crate::domain::equality_join::types::{ProbeParts, SeededJoin};
use crate::domain::ordered_dfs::types::DfsParts;
use crate::domain::physical_plan::types::{IndexPlace, ProbeIndex, RowMap, RowSource};
use crate::domain::project::types::ProjectParts;
use crate::domain::select::types::SelectParts;
use crate::domain::worklist::types::WorklistParts;
use ryo_relations::domain::relations::types::TextHash;

/// The kernels one node is lowered to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Lowering {
    /// `filter`: the rows of the source the comparisons hold for, with the kept columns.
    Filter {
        /// What it reads.
        source: RowSource,
        /// The comparisons, the kept columns and the bindings.
        select: SelectParts,
    },
    /// `lookup` of a constant in a relation's shared `hash_index`, then `filter` of all the select's comparisons.
    Lookup {
        /// The index it reads.
        index: IndexPlace,
        /// The hash of the constant the key equals.
        key: TextHash,
        /// The comparisons, the kept columns and the bindings.
        select: SelectParts,
    },
    /// `project`: the columns in order, repeated rows dropped when asked.
    Project {
        /// What it reads.
        source: RowSource,
        /// The columns and whether a repeated row is kept once.
        project: ProjectParts,
    },
    /// `extend`: each row followed by the values, resolved once per file.
    Extend {
        /// What it reads.
        source: RowSource,
        /// The values and what their parameters are bound to.
        extension: ExtendParts,
    },
    /// `hash_index` of the right rows, then `probe` of it with each left row.
    Probe {
        /// The rows probed with, in their order.
        left: RowSource,
        /// The rows indexed.
        right: RowSource,
        /// What the probe keeps.
        probe: ProbeParts,
        /// The index it probes: its own, or the one every probe of its right rows and key shares.
        index: ProbeIndex,
    },
    /// `distinct` over the rows of each source in turn: a row the first time only.
    Distinct {
        /// What it reads, in order.
        sources: [RowSource; 2],
    },
    /// `group`: the rows keyed through `hash_index`'s table, one row per group in the order of its first row, with its count.
    Group {
        /// What it reads.
        source: RowSource,
        /// The keys and what is counted.
        group: GroupParts,
    },
    /// `reach` from every source (`traverse::All`): one row per partition, source and entity it reaches.
    Reach {
        /// The edge rows.
        edges: RowSource,
        /// The graph they make.
        graph: ReachParts,
    },
    /// `reach` from the sources the left rows bind (`traverse::Seeds`), probed by them: a closure the planner fused into
    /// the join that keys it by its partition and source. Each bound source is walked once per file.
    SeededReach {
        /// The left rows, read in order.
        left: RowSource,
        /// The edge rows.
        edges: RowSource,
        /// The graph they make.
        graph: ReachParts,
        /// The join over the reached rows.
        join: SeededJoin,
    },
    /// `directed_walk`: from each start row, the entities a walk over the edges reaches in a direction that the domain
    /// admits.
    Axis {
        /// The start rows, read in order.
        starts: RowSource,
        /// The edge rows.
        edges: RowSource,
        /// The domain rows.
        domain: RowSource,
        /// The walk.
        axis: AxisParts,
    },
    /// `ordered_walk`: from each distinct partition and root, in sorted order, the edges an ordered depth-first visit reaches.
    OrderedWalk {
        /// The root rows.
        roots: RowSource,
        /// The edge rows.
        edges: RowSource,
        /// The visit.
        dfs: DfsParts,
    },
    /// `worklist`: from each distinct partition and root, in sorted order, the edges a first-in-first-out dataflow follows.
    Worklist {
        /// The root rows.
        roots: RowSource,
        /// The edge rows.
        edges: RowSource,
        /// The lattice rows.
        lattice: RowSource,
        /// The worklist.
        worklist: WorklistParts,
    },
    /// `components`: each entity of a strongly connected component of more than one, or with an edge to itself, once.
    Components {
        /// The edge rows.
        edges: RowSource,
        /// The graph they make.
        cycles: CycleParts,
    },
    /// `text_map`: each row of the source through one text kind's per-row meaning, giving none, one or many rows.
    TextMap {
        /// What it reads.
        source: RowSource,
        /// The kind's parts.
        map: RowMap,
    },
    /// `ordered_match`: both sources ordered by key then order, each left row paired with the earliest later right row.
    OrderedMatch {
        /// The left rows.
        left: RowSource,
        /// The right rows.
        right: RowSource,
        /// The keys, the orders and the output.
        ordered: crate::domain::ordered_match::types::OrderedParts,
    },
    /// `render`: one batch of renderings for every row of the source, refused whole on one unsupported answer.
    Render {
        /// What it reads.
        source: RowSource,
        /// The node column and the role.
        render: crate::domain::render_batch::types::RenderParts,
    },
    /// `scan_bound`: the rows the engine holds for the run, or a table parameter's rows.
    ScanBound {
        /// What it scans.
        scan: BoundScan,
    },
    /// No kernel at this stage: the plan's `unserved` names the stage that lowers it.
    Unlowered,
}

/// What a bound scan reads: the host's rows of a declared relation, or the tables a `list<table>` parameter is bound to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BoundScan {
    /// The host's rows of a declared relation.
    Hosted(ryo_values::domain::plan_parts::types::HostedNumber),
    /// One row for each table of a parameter, the text under each key in key order.
    Rows {
        /// The parameter.
        parameter: ryo_values::domain::plan_parts::types::ParameterId,
        /// The keys, in order.
        keys: Vec<ryo_values::domain::plan_parts::types::TableKey>,
        /// What the node's parameters are bound to.
        bindings: Vec<ryo_values::domain::query_bindings::types::ParameterBinding>,
    },
}

impl Lowering {
    /// The kind of its kernels, as the node counters report it.
    pub(crate) fn kind(
        &self,
    ) -> crate::domain::physical_plan::types::node_counters::value::NodeKind {
        use crate::domain::physical_plan::types::node_counters::value::NodeKind;
        match self {
            Self::Filter { .. } => NodeKind::Filter,
            Self::Lookup { .. } => NodeKind::Lookup,
            Self::Project { .. } => NodeKind::Project,
            Self::Extend { .. } => NodeKind::Extend,
            Self::Probe { .. } => NodeKind::Probe,
            Self::Distinct { .. } => NodeKind::Distinct,
            Self::Group { .. } => NodeKind::Group,
            Self::Reach { .. } => NodeKind::Reach,
            Self::SeededReach { .. } => NodeKind::SeededReach,
            Self::Axis { .. } => NodeKind::Axis,
            Self::OrderedWalk { .. } => NodeKind::OrderedWalk,
            Self::Worklist { .. } => NodeKind::Worklist,
            Self::Components { .. } => NodeKind::Components,
            Self::TextMap { .. } => NodeKind::TextMap,
            Self::OrderedMatch { .. } => NodeKind::OrderedMatch,
            Self::Render { .. } => NodeKind::Render,
            Self::ScanBound { .. } => NodeKind::ScanBound,
            Self::Unlowered => NodeKind::Unlowered,
        }
    }

    /// The rows the node reads, in the order its kernels read them; a lookup reads its index's relation, which is not here.
    pub(crate) fn sources(&self) -> impl Iterator<Item = RowSource> {
        let (first, second, third) = match self {
            Self::Filter { source, .. }
            | Self::Project { source, .. }
            | Self::Extend { source, .. }
            | Self::Group { source, .. }
            | Self::TextMap { source, .. }
            | Self::Render { source, .. } => (Some(*source), None, None),
            Self::Reach { edges, .. } | Self::Components { edges, .. } => {
                (Some(*edges), None, None)
            }
            Self::Probe { left, right, .. } | Self::OrderedMatch { left, right, .. } => {
                (Some(*left), Some(*right), None)
            }
            Self::Distinct {
                sources: [left, right],
            } => (Some(*left), Some(*right), None),
            Self::SeededReach { left, edges, .. } => (Some(*left), Some(*edges), None),
            Self::OrderedWalk { roots, edges, .. } => (Some(*roots), Some(*edges), None),
            Self::Axis {
                starts,
                edges,
                domain,
                ..
            } => (Some(*starts), Some(*edges), Some(*domain)),
            Self::Worklist {
                roots,
                edges,
                lattice,
                ..
            } => (Some(*roots), Some(*edges), Some(*lattice)),
            Self::Lookup { .. } | Self::ScanBound { .. } | Self::Unlowered => (None, None, None),
        };
        first.into_iter().chain(second).chain(third)
    }
}
