//! The `Lowering` value: the kernels one node of the physical plan is lowered to, and the rows they read.
use crate::domain::aggregate::types::GroupParts;
use crate::domain::append::types::ExtendParts;
use crate::domain::equality_join::types::ProbeParts;
use crate::domain::physical_plan::types::{IndexPlace, RowSource};
use crate::domain::project::types::ProjectParts;
use crate::domain::select::types::SelectParts;
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
    /// `lookup` of a constant in the shared `hash_index` of a relation's column, then `filter` of all the select's
    /// comparisons: the rows whose key's text is the constant, in source row order.
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
    /// No kernel at this stage: the plan's `unserved` names the stage that lowers it.
    Unlowered,
}

impl Lowering {
    /// The rows the node reads, in the order its kernels read them; a lookup reads its index's relation, which is not here.
    pub(crate) fn sources(&self) -> impl Iterator<Item = RowSource> {
        let (first, second) = match self {
            Self::Filter { source, .. }
            | Self::Project { source, .. }
            | Self::Extend { source, .. }
            | Self::Group { source, .. } => (Some(*source), None),
            Self::Probe { left, right, .. } => (Some(*left), Some(*right)),
            Self::Distinct { sources: [left, right] } => (Some(*left), Some(*right)),
            Self::Lookup { .. } | Self::Unlowered => (None, None),
        };
        first.into_iter().chain(second)
    }
}
