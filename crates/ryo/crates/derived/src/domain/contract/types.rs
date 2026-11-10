//! The types of the `contract` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::adjacency::types::compact_graph::value::CompactGraph;
use crate::domain::closure_adjacency::types::ClosurePartition;
use crate::domain::cycle_adjacency::types::CycleGraph;
use crate::domain::dfs::types::PreparedDfs;
use crate::domain::equality::types::KeyIndex;
use crate::domain::membership::types::{MarkSpace, MembershipView};
use crate::domain::native::types::{NativeFinding, NativeForm};
use crate::domain::ordered::types::OrderedMatching;
use crate::domain::row_cells::types::{CanonicalTexts, RowCells, ValueTexts};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::rows::types::RowIndex;

/// Unsupported readable native/oversized layouts decline distinctly from faults.
#[derive(Debug)]
pub enum BuildOutcome {
    /// The payload was built.
    Built(derived_payload::value::DerivedPayload),
    /// The layout is unsupported; the caller falls back.
    Declined,
}

/// Borrowing an existing payload cannot build, acquire inputs, or mutate its storage.
pub enum DerivedView<'a> {
    /// The key index.
    Equality(&'a KeyIndex),
    /// The compact graph.
    Adjacency(&'a CompactGraph),
    /// Membership.
    Membership(MembershipView<'a>),
    /// The typed partition order.
    PartitionOrder(&'a [RowIndex]),
    /// A closure partition.
    ClosureAdjacency(&'a ClosurePartition),
    /// The cycle graph.
    CycleAdjacency(&'a CycleGraph),
    /// The ordered matching.
    OrderedKeys(&'a OrderedMatching),
    /// The prepared DFS.
    OrderedDfs(&'a PreparedDfs),
    /// The native finding.
    NativeLookup(&'a NativeFinding),
}

/// The finite inputs of one build, by kind.
pub enum BuildInputs<'a> {
    /// Rows.
    Rows(&'a dyn RowCells),
    /// Rows and the space to mark members in.
    Membership {
        /// The rows.
        rows: &'a dyn RowCells,
        /// The space to mark.
        space: MarkSpace<'a>,
    },
    /// Rows of one partition.
    Partition {
        /// The rows.
        rows: &'a dyn RowCells,
        /// The positions of the partition's rows.
        positions: Option<&'a [RowIndex]>,
        /// The partition key.
        key: &'a [Value],
    },
    /// Both sides of an ordered matching.
    Ordered {
        /// The left rows.
        left: &'a dyn RowCells,
        /// The right rows.
        right: &'a dyn RowCells,
        /// The texts of symbols and names.
        texts: &'a dyn ValueTexts,
    },
    /// Roots and edges of a depth-first lookup.
    Dfs {
        /// The roots.
        roots: &'a dyn RowCells,
        /// The edges.
        edges: &'a dyn RowCells,
        /// The texts, which may intern canonical forms.
        texts: &'a mut dyn CanonicalTexts,
    },
    /// The native table to look in.
    Native(NativeForm),
}

/// Scratch the caller lends one build.
#[derive(Default)]
pub struct BuildWorkspace {
    /// A row buffer reused across the build.
    pub row: Vec<Value>,
}

pub mod prepared_derived;

pub mod derived_payload;
