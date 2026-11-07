//! Read-only physical inputs expose no owned cache or retained selection handle.

use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_operators::domain::set_rows::types::RightMembership;
use ryo_operators::domain::source_matches::types::{BaseLine, SourceBase};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::row_texts::types::CellText;

use crate::domain::table_model::types::RepresentationInput;

/// Profile-only semantic observations. No execution or ownership policy is reachable.
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy)]
pub(crate) enum ProgressCounter {
    InputRows,
    PredicateEvaluations,
    IndexProbes,
    IndexRowsExamined,
    EqualityMatches,
    JoinCandidatePairs,
    KernelOutputRows,
}
/// How much one profile observation adds to its counter: the bare count inside is named here.
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProgressAmount(pub u64);
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) trait ProgressObserver: Sync {
    /// Adds `amount` to what `counter` has counted.
    fn observe(&self, counter: ProgressCounter, amount: ProgressAmount);
}
/// The observer of a run that profiles nothing.
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) struct IgnoredProgress;
#[cfg(feature = "diagnostic-row-progress")]
impl ProgressObserver for IgnoredProgress {
    fn observe(&self, _counter: ProgressCounter, _amount: ProgressAmount) {}
}

/// Exact typed key lookup prepared by the engine; primitives only probe it.
#[derive(Clone, Copy)]
pub(crate) struct IndexedRows<'a> {
    #[cfg(feature = "diagnostic-row-progress")]
    pub progress: &'a dyn ProgressObserver,
    pub rows: RepresentationInput<'a>,
    pub columns: &'a [ColumnIndex],
    pub index: &'a KeyIndex,
}
impl IndexedRows<'_> {
    pub(crate) fn contains(&self, key: &[Value]) -> RightMembership {
        RightMembership::from(self.matching(key).next().is_some())
    }
    pub(crate) fn probe(&self, key: &[Value]) -> KeyProbe {
        #[cfg(feature = "diagnostic-row-progress")]
        self.progress.observe(ProgressCounter::IndexProbes, ProgressAmount(1));
        let hash = ryo_derived::domain::key::service::hash_key(key.iter().map(Some));
        let bucket =
            usize::try_from(hash.0).unwrap_or(0) & self.index.heads.len().saturating_sub(1);
        KeyProbe {
            hash,
            at: self
                .index
                .heads
                .get(bucket)
                .copied()
                .unwrap_or(ryo_derived::domain::key::types::ChainLink(0)),
        }
    }
    pub(crate) fn next_match(&self, key: &[Value], probe: &mut KeyProbe) -> Option<RowIndex> {
        use ryo_derived::domain::key::types::ChainLink;
        while probe.at.0 != 0 {
            #[cfg(feature = "diagnostic-row-progress")]
            self.progress.observe(ProgressCounter::IndexRowsExamined, ProgressAmount(1));
            let position = usize::try_from(probe.at.0 - 1).unwrap_or(usize::MAX);
            probe.at = self
                .index
                .next
                .get(position)
                .copied()
                .unwrap_or(ChainLink(0));
            let row = self.index.rows.get(position).copied()?;
            if self.index.hashes.get(position) == Some(&probe.hash)
                && self
                    .columns
                    .iter()
                    .zip(key)
                    .all(|(column, expected)| self.rows.cell(row, *column) == Some(*expected))
            {
                #[cfg(feature = "diagnostic-row-progress")]
                self.progress.observe(ProgressCounter::EqualityMatches, ProgressAmount(1));
                return Some(row);
            }
        }
        None
    }
    pub(crate) fn matching<'a>(&'a self, key: &'a [Value]) -> MatchingRows<'a> {
        MatchingRows {
            view: self,
            key,
            probe: self.probe(key),
        }
    }
}
/// Physical hash-chain position, independent of the caller-owned key buffer.
pub(crate) struct KeyProbe {
    hash: ryo_derived::domain::key::types::KeyCellsHash,
    at: ryo_derived::domain::key::types::ChainLink,
}
pub(crate) struct MatchingRows<'a> {
    view: &'a IndexedRows<'a>,
    key: &'a [Value],
    probe: KeyProbe,
}
impl Iterator for MatchingRows<'_> {
    type Item = RowIndex;
    fn next(&mut self) -> Option<RowIndex> {
        self.view.next_match(self.key, &mut self.probe)
    }
}

use ryo_values::domain::rows::types::RowIndex;

/// Finite immutable partition access; ownership and recycling remain in the invocation owner.
pub(crate) trait ClosureParts {
    fn len(&self) -> PartitionCount;
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
pub(crate) struct PartitionCount(pub usize);

/// One partition of a closure, by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PartitionIndex(pub usize);

#[derive(Clone, Copy, Default)]
pub(crate) enum ClosureViews<'a> {
    #[default]
    Empty,
    One(&'a ryo_derived::domain::closure_adjacency::types::ClosurePartition),
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
pub(crate) struct ClosureIter<'a> {
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
pub(crate) trait PredicateProgram {
    fn holds(
        &mut self,
        row: &[Value],
        symbols: &ryo_language::domain::symbols::types::Symbols,
    ) -> Result<Verdict, OperatingError>;
}

/// The checked physical text of a source-matching span, with where it starts and the line it begins on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ValidatedSource<'a> {
    pub base: SourceBase,
    pub text: CellText<'a>,
    pub line: BaseLine,
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
pub(crate) fn validate<'a>(
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

/// A native lookup bound to a join: the tree it finds candidates in, and how it finds them.
#[derive(Clone, Copy)]
pub(crate) struct NativeJoin<'a> {
    pub tree: crate::domain::table_cells::types::TreeRows<'a>,
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
pub(crate) enum PreparedAxis<'a> {
    Empty,
    Native {
        tree: crate::domain::table_cells::types::TreeRows<'a>,
        among: Option<crate::domain::table_cells::types::TreeRows<'a>>,
        marks: ryo_derived::domain::positions::types::DenseMarks<'a>,
    },
    Indexed {
        edges: IndexedRows<'a>,
        to: ryo_values::domain::plan_parts::types::ColumnIndex,
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
