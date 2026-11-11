//! The types of the `views` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_reach::borrowed::{types::IndexedRows, types::KeyProbe};
use ryo_operators::domain::cycles::types::{CycleDriver, EdgePlace, NodeCount, NodeNumber};
use ryo_operators::domain::source_matches::types::{BaseLine, SourceBase};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::row_texts::types::CellText;

/// Finite immutable partition access; ownership and recycling remain in the invocation owner.
pub trait ClosureParts {
    /// How many partitions there are.
    fn len(&self) -> PartitionCount;
    /// One partition, by its place.
    ///
    /// # Errors
    ///
    /// The [`OperatingError`] of a place outside the partitions,
    /// or of a partition that cannot be read.
    fn get(
        &self,
        at: PartitionIndex,
    ) -> Result<&ryo_derived::domain::closure_adjacency::types::ClosurePartition, OperatingError>;
}

/// How many partitions a closure holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionCount(pub usize);

/// One partition of a closure, by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionIndex(pub usize);

/// The closure partitions an operation reads.
#[derive(Clone, Copy, Default)]
pub enum ClosureViews<'a> {
    /// No partition.
    #[default]
    Empty,
    /// One partition.
    One(&'a ryo_derived::domain::closure_adjacency::types::ClosurePartition),
    /// Several partitions, read through their owner.
    Many(&'a dyn ClosureParts),
}

/// The partitions of a closure, one at a time.
pub struct ClosureIter<'a> {
    pub views: ClosureViews<'a>,
    pub at: PartitionIndex,
}

impl<'a> Iterator for ClosureIter<'a> {
    type Item =
        Result<&'a ryo_derived::domain::closure_adjacency::types::ClosurePartition, OperatingError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.at.0 >= crate::domain::operator::contract::views::service::len(self.views).0 {
            return None;
        }
        let at = self.at;
        self.at.0 += 1;
        Some(crate::domain::operator::contract::views::service::get(
            self.views, at,
        ))
    }
}

impl<'a> IntoIterator for ClosureViews<'a> {
    type Item = <ClosureIter<'a> as Iterator>::Item;
    type IntoIter = ClosureIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ClosureIter {
            views: self,
            at: PartitionIndex(0),
        }
    }
}

/// The evaluator is bound to the prepared contract's exact ordered program.
/// Its implementation is engine-private; this interface cannot choose predicates,
/// schemas, cache keys or upstream scheduling.
pub trait PredicateProgram {
    /// Whether the predicate holds for a row.
    ///
    /// # Errors
    ///
    /// The [`OperatingError`] of the first comparison that cannot be decided over the row.
    fn holds(
        &mut self,
        row: &[Value],
        symbols: &ryo_language::domain::symbols::types::Symbols,
    ) -> Result<Verdict, OperatingError>;
}

/// The checked physical text of a source-matching span, with where it starts and the line it begins on.
#[derive(Clone, Copy, Debug)]
pub struct ValidatedSource<'a> {
    pub(crate) base: SourceBase,
    pub(crate) text: CellText<'a>,
    pub(crate) line: BaseLine,
}

/// The checked physical text of a span and where it starts in the source.
pub(super) struct SourceSlice<'a> {
    pub(crate) base: SourceBase,
    pub(crate) text: CellText<'a>,
}

/// A native lookup bound to a join: the tree it finds candidates in, and how it finds them.
#[derive(Clone, Copy, Debug)]
pub struct NativeJoin<'a> {
    /// The tree the lookup finds candidates in.
    pub tree: crate::domain::table_cells::types::TreeRows<'a>,
    /// How the lookup finds them.
    pub finding: ryo_derived::domain::native::types::NativeFinding,
}

/// Whether the native candidates of the left row have been found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Started {
    Yes,
    No,
}

/// Where the walk over the native candidates has reached.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NativeCursor(pub(crate) usize);

/// The join candidates of one left row: the native tree candidates when a native lookup is bound, else the
/// index probe; a filtered native join admits only the elements its bound predicate holds for.
pub(crate) struct JoinCursor<'f> {
    pub(crate) native: Option<NativeJoin<'f>>,
    pub(crate) index: Option<IndexedRows<'f>>,
    pub(crate) probe: Option<KeyProbe>,
    pub(crate) current: Option<ryo_values::domain::rows::types::RowIndex>,
    pub(crate) positions: Vec<ryo_relations::domain::relations::types::Position>,
    pub(crate) at: NativeCursor,
    pub(crate) started: Started,
    pub(crate) position: Option<ryo_relations::domain::relations::types::Position>,
    pub(crate) filtered: super::super::resources::types::Filtering,
}

/// The axis resources one walk runs over: none, a tree of elements, or an index of edges with its domain.
#[derive(Clone, Copy)]
pub enum PreparedAxis<'a> {
    /// No resource.
    Empty,
    /// A tree of elements.
    Native {
        /// The tree walked.
        tree: crate::domain::table_cells::types::TreeRows<'a>,
        /// The nodes the walk is confined to, when it is confined.
        among: Option<crate::domain::table_cells::types::TreeRows<'a>>,
        /// The marks of the domain's members.
        marks: ryo_derived::domain::positions::types::DenseMarks<'a>,
    },
    /// An index of edges with its domain.
    Indexed {
        /// The indexed edges.
        edges: IndexedRows<'a>,
        /// The column edges lead to.
        to: ryo_values::domain::plan_parts::types::ColumnIndex,
        /// The members of the domain.
        domain: ryo_derived::domain::membership::types::MembershipView<'a>,
    },
}

/// The axis resources the last walk ran over.
#[derive(Default)]
pub(crate) struct AxisCursor<'f> {
    pub(crate) prepared: Option<PreparedAxis<'f>>,
}

/// The prepared adjacency of the derived resource, read through the operators' port.
pub(crate) struct PreparedAdjacency<'a>(
    pub(crate) &'a ryo_derived::domain::cycle_adjacency::types::CycleGraph,
);

/// The graph resources an evaluation acquired: its ordered keys, adjacency, closure partitions and depth-first
/// graph.
#[derive(Default)]
pub(crate) struct GraphCursor<'f> {
    pub(crate) ordered: Option<&'f ryo_derived::domain::ordered::types::OrderedMatching>,
    pub(crate) cycles: Option<PreparedAdjacency<'f>>,
    pub(crate) closure: ClosureViews<'f>,
    pub(crate) dfs: Option<&'f ryo_derived::domain::dfs::types::PreparedDfs>,
}

impl CycleDriver for PreparedAdjacency<'_> {
    fn nodes(&self) -> NodeCount {
        NodeCount(self.0.values.len())
    }
    fn value(&self, node: NodeNumber) -> Option<Value> {
        self.0.values.get(node.0).copied()
    }
    fn edge(&self, node: NodeNumber, place: EdgePlace) -> Option<NodeNumber> {
        super::super::graph::service::adjacency_edge(self, node, place)
    }
}
