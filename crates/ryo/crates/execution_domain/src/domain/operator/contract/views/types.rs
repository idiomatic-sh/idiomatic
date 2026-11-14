//! The types of the `views` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::contract::types::PredicateOutcome;
use crate::domain::table_reach::borrowed::types::IndexedRows;
use ryo_operators::domain::cycles::types::{CycleDriver, EdgePlace, NodeCount, NodeNumber};
use ryo_operators::domain::source_matches::types::{BaseLine, SourceBase};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::row_texts::types::CellText;

/// Finite immutable partition access; ownership and recycling remain in the invocation owner.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::views::types::ClosureParts;
/// fn bound<T: ClosureParts + ?Sized>(_: &T) {}
/// ```
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
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::views::types::PartitionCount;
/// fn named(_: Option<&PartitionCount>) {}
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionCount(pub usize);

/// One partition of a closure, by its place among them.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::views::types::PartitionIndex;
/// fn named(_: Option<&PartitionIndex>) {}
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionIndex(pub usize);

/// The closure partitions an operation reads.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::views::types::ClosureViews;
/// fn named(_: Option<&ClosureViews<'_>>) {}
/// ```
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

/// The evaluator is bound to the prepared contract's exact ordered program.
/// Its implementation is engine-private; this interface cannot choose predicates,
/// schemas, cache keys or upstream scheduling.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::views::types::PredicateProgram;
/// fn bound<T: PredicateProgram + ?Sized>(_: &T) {}
/// ```
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
    ) -> Result<PredicateOutcome, OperatingError>;
}

/// The checked physical text of a source-matching span, with where it starts and the line it begins on.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::views::types::ValidatedSource;
/// fn named(_: Option<&ValidatedSource<'_>>) {}
/// ```
///
/// Only the source gate's check makes one: no other crate can claim a span is checked.
///
/// ```compile_fail
/// use ryo_execution_domain::domain::operator::contract::views::types::ValidatedSource;
/// use ryo_operators::domain::source_matches::types::{BaseLine, SourceBase};
/// use ryo_values::domain::row_texts::types::CellText;
/// let forged = ValidatedSource {
///     base: SourceBase(0),
///     text: CellText(""),
///     line: BaseLine::Unnumbered,
/// };
/// ```
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
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::views::types::NativeJoin;
/// fn named(_: Option<&NativeJoin<'_>>) {}
/// ```
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

/// The axis resources one walk runs over: none, a tree of elements, or an index of edges with its domain.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::views::types::PreparedAxis;
/// fn named(_: Option<&PreparedAxis<'_>>) {}
/// ```
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

/// The prepared adjacency of the derived resource, read through the operators' port.
pub(crate) struct PreparedAdjacency<'a>(
    pub(crate) &'a ryo_derived::domain::cycle_adjacency::types::CycleGraph,
);

impl CycleDriver for PreparedAdjacency<'_> {
    fn nodes(&self) -> NodeCount {
        NodeCount(self.0.values.len())
    }
    fn value(&self, node: NodeNumber) -> Option<Value> {
        self.0.values.get(node.0).copied()
    }
    fn edge(&self, node: NodeNumber, place: EdgePlace) -> Option<NodeNumber> {
        let starts = &self.0.starts.0;
        let at = starts.get(node.0)?.checked_add(place.0)?;
        (at < *starts.get(node.0 + 1)?)
            .then(|| self.0.leaving.0.get(at).copied().map(NodeNumber))
            .flatten()
    }
}
