//! Read-only physical inputs expose no owned cache or retained selection handle.

use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use ryo_operators::domain::source_matches::types::{BaseLine, SourceBase};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::row_texts::types::CellText;

use crate::domain::table_reach::borrowed::{IndexedRows, KeyProbe};

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
    ) -> Result<
        &ryo_derived::domain::closure_adjacency::types::ClosurePartition,
        crate::domain::operating::errors::OperatingError,
    >;
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
impl<'a> ClosureViews<'a> {
    pub(crate) fn len(self) -> PartitionCount {
        match self {
            Self::Empty => PartitionCount(0),
            Self::One(_) => PartitionCount(1),
            Self::Many(parts) => parts.len(),
        }
    }
    pub(crate) fn get(
        self,
        at: PartitionIndex,
    ) -> Result<
        &'a ryo_derived::domain::closure_adjacency::types::ClosurePartition,
        crate::domain::operating::errors::OperatingError,
    > {
        match self {
            Self::Empty => Err(crate::domain::operating::errors::OperatingError::ShapeMismatch),
            Self::One(part) => (at == PartitionIndex(0))
                .then_some(part)
                .ok_or(crate::domain::operating::errors::OperatingError::ShapeMismatch),
            Self::Many(parts) => parts.get(at),
        }
    }
}
/// The partitions of a closure, one at a time.
pub struct ClosureIter<'a> {
    views: ClosureViews<'a>,
    at: PartitionIndex,
}
impl<'a> Iterator for ClosureIter<'a> {
    type Item = Result<
        &'a ryo_derived::domain::closure_adjacency::types::ClosurePartition,
        crate::domain::operating::errors::OperatingError,
    >;
    fn next(&mut self) -> Option<Self::Item> {
        if self.at.0 >= self.views.len().0 {
            return None;
        }
        let at = self.at;
        self.at.0 += 1;
        Some(self.views.get(at))
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
struct SourceSlice<'a> {
    base: SourceBase,
    text: CellText<'a>,
}

/// Returns the physical text only after validating the whole input span. An absent span carries no
/// occurrences; malformed boundaries fail before the row emits.
fn source_slice<'a>(
    row: &[Value],
    span: ColumnIndex,
    source: ryo_values::domain::files::types::SourceView<'a>,
) -> Result<Option<SourceSlice<'a>>, OperatingError> {
    let Value::Span(span) = ryo_values::domain::rows::service::cell(row, span)? else {
        return Err(OperatingError::ShapeMismatch);
    };
    if span == ryo_relations::domain::relations::constants::NO_SPAN {
        return Ok(None);
    }
    let start =
        usize::try_from(span.start.0).map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
    let end = usize::try_from(span.end.0).map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
    if start > end {
        return Err(OperatingError::InvalidSourceMatchSpan);
    }
    let bytes = source
        .0
        .get(start..end)
        .ok_or(OperatingError::InvalidSourceMatchSpan)?;
    let text =
        std::str::from_utf8(bytes).map_err(|_error| OperatingError::InvalidSourceMatchSpan)?;
    Ok(Some(SourceSlice {
        base: SourceBase(start),
        text: CellText(text),
    }))
}

/// The source gate of an occurrence search: validates the span, and its base line, before any matcher is
/// acquired. An absent span suppresses the row's later checks.
///
/// # Errors
///
/// [`OperatingError::ShapeMismatch`] when the span or base-line cell is of another kind, and
/// [`OperatingError::InvalidSourceMatchSpan`] for a span out of order, outside the source or off a character
/// boundary, or a base line that is not positive; a missing column is the row's own refusal.
pub fn validate<'a>(
    row: &[Value],
    span: ColumnIndex,
    base_line: Option<ColumnIndex>,
    source: ryo_values::domain::files::types::SourceView<'a>,
) -> Result<Option<ValidatedSource<'a>>, OperatingError> {
    let Some(SourceSlice { base, text }) = source_slice(row, span, source)? else {
        return Ok(None);
    };
    let line = if let Some(column) = base_line {
        let Value::Number(ryo_relations::domain::relations::types::Integer(number)) =
            ryo_values::domain::rows::service::cell(row, column)?
        else {
            return Err(OperatingError::ShapeMismatch);
        };
        if number <= 0 {
            return Err(OperatingError::InvalidSourceMatchSpan);
        }
        BaseLine::Starts(ryo_relations::domain::relations::types::Integer(number))
    } else {
        BaseLine::Unnumbered
    };
    Ok(Some(ValidatedSource { base, text, line }))
}

/// Admits a file's population of a provider relation for the source searches that rely on its spans: every cell of the
/// named span columns must pass the occurrence search's own span gate, so a search over them cannot fail on its span.
///
/// # Errors
///
/// The first refusal the gate meets: [`OperatingError::InvalidSourceMatchSpan`] for a span out of order, outside the
/// source or off a character boundary, [`OperatingError::ShapeMismatch`] for a cell that is no span, and
/// [`OperatingError::NotRead`] for a row the population cannot read.
pub fn validate_spans(
    rows: &crate::domain::table_model::types::Rows<'_>,
    columns: &[ColumnIndex],
    source: ryo_values::domain::files::types::SourceView<'_>,
) -> Result<(), OperatingError> {
    let mut cells = Vec::new();
    for index in ryo_values::domain::rows::service::positions(
        crate::domain::table_model::service::count_of(rows),
    ) {
        let row = crate::domain::table_model::service::read_row(rows, index, &mut cells)
            .ok_or(OperatingError::NotRead)?;
        for column in columns {
            let _source = source_slice(row, *column, source)?;
        }
    }
    Ok(())
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
pub(super) enum Started {
    Yes,
    No,
}

/// Where the walk over the native candidates has reached.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct NativeCursor(pub(super) usize);

/// The join candidates of one left row: the native tree candidates when a native lookup is bound, else the
/// index probe; a filtered native join admits only the elements its bound predicate holds for.
pub(crate) struct JoinCursor<'f> {
    pub(super) native: Option<NativeJoin<'f>>,
    pub(super) index: Option<IndexedRows<'f>>,
    pub(super) probe: Option<KeyProbe>,
    pub(super) current: Option<ryo_values::domain::rows::types::RowIndex>,
    pub(super) positions: Vec<ryo_relations::domain::relations::types::Position>,
    pub(super) at: NativeCursor,
    pub(super) started: Started,
    pub(super) position: Option<ryo_relations::domain::relations::types::Position>,
    pub(super) filtered: super::resources::Filtering,
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
    pub(super) prepared: Option<PreparedAxis<'f>>,
}

/// The prepared adjacency of the derived resource, read through the operators' port.
pub(super) struct PreparedAdjacency<'a>(
    pub(super) &'a ryo_derived::domain::cycle_adjacency::types::CycleGraph,
);

/// The graph resources an evaluation acquired: its ordered keys, adjacency, closure partitions and depth-first
/// graph.
#[derive(Default)]
pub(crate) struct GraphCursor<'f> {
    pub(super) ordered: Option<&'f ryo_derived::domain::ordered::types::OrderedMatching>,
    pub(super) cycles: Option<PreparedAdjacency<'f>>,
    pub(super) closure: ClosureViews<'f>,
    pub(super) dfs: Option<&'f ryo_derived::domain::dfs::types::PreparedDfs>,
}
