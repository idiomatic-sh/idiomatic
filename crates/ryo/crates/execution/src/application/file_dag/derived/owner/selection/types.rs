//! Immutable selector choices and candidate ownership; no scheduling or builds.
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;
// One owner bounds the actual shared position allocations and retained predicate entries.
pub(in crate::application::file_dag::derived::owner) const MAX_RETAINED_SELECTIONS: usize = 128;
pub(in crate::application::file_dag::derived::owner) const MAX_RETAINED_POSITION_CAPACITY: usize =
    512 * 1024;
// A single pass can route several authored literals without a whole-column key index.
// Existing fanout count, text and position bounds remain independent of byte admission.
pub(in crate::application::file_dag::derived::owner) const MAX_FANOUT_POPULATION: usize =
    256 * 1024;
pub(in crate::application::file_dag::derived::owner) const MAX_FANOUT_LITERALS: usize = 128;
pub(in crate::application::file_dag::derived::owner) const MAX_FANOUT_GROUPS: usize = 4;
pub(in crate::application::file_dag::derived::owner) const MAX_FANOUT_TEXT_BYTES: usize = 64 * 1024;
pub(in crate::application::file_dag::derived::owner) const MIN_FANOUT_LITERALS: usize = 8;

/// Diagnostic reasons are fixed integers so normal selection never formats strings.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::application::file_dag::derived::owner) enum SelectionPath {
    NoDiscriminator = 0,
    NotRepeated = 1,
    SmallPopulation = 2,
    DenseSample = 3,
    BudgetRefused = 4,
    IndexBuilt = 5,
    IndexReused = 6,
    FanoutBuilt = 7,
    FanoutReused = 8,
}

/// Row positions this selection owns outright.
pub(in crate::application::file_dag::derived::owner) struct CandidatePositions(pub Vec<u32>);

/// Row positions shared through an admitted derived view.
#[derive(Clone)]
pub(in crate::application::file_dag::derived::owner) struct SelectedPositions(pub Arc<Vec<u32>>);

/// Borrowed candidate row positions.
pub(in crate::application::file_dag::derived::owner) struct CandidateSlice<'a>(pub &'a [u32]);

pub(in crate::application::file_dag::derived::owner) enum CandidateRows {
    Owned(CandidatePositions),
    Shared {
        rows: SelectedPositions,
        _resource: crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    },
}

impl CandidateRows {
    pub(in crate::application::file_dag::derived::owner) fn as_slice(&self) -> CandidateSlice<'_> {
        match self {
            Self::Owned(rows) => CandidateSlice(&rows.0),
            Self::Shared { rows, .. } => CandidateSlice(rows.0.as_ref().as_slice()),
        }
    }
}

/// The candidate rows chosen for a selector, if any, and why that path was taken.
pub(in crate::application::file_dag::derived::owner) struct CandidateChoice {
    pub rows: Option<CandidateRows>,
    pub path: SelectionPath,
}

/// Candidates a single fanout pass produced, with the view that keeps them alive.
pub(in crate::application::file_dag::derived::owner) struct FanoutHit {
    pub rows: SelectedPositions,
    pub resource:
        crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    pub path: SelectionPath,
}

/// Whether an index could be retained within the cache budget.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::application::file_dag::derived::owner) enum Retainability {
    Retainable,
    Unretainable,
}

/// Whether two selectors share relation, ordered predicates and bindings.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::application::file_dag::derived::owner) enum SelectorMatch {
    Same,
    Different,
}

/// Whether a bounded sample shows a discriminator selects few enough rows to index.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::application::file_dag::derived::owner) enum Selectivity {
    Selective,
    Dense,
}

/// Rows of the physical population a selector reads.
#[derive(Clone, Copy)]
pub(in crate::application::file_dag::derived::owner) struct RowPopulation(pub usize);

/// Retained bytes the cache accounts to one prepared structure.
#[derive(Clone, Copy)]
pub(in crate::application::file_dag::derived::owner) struct PostingBytes(pub usize);

/// The authored text of a first-literal equality.
#[derive(Clone, Copy)]
pub(in crate::application::file_dag::derived::owner) struct LiteralSpelling<'a>(pub &'a str);

/// A total first-literal equality: its column and authored text.
pub(in crate::application::file_dag::derived::owner) struct FirstLiteral<'a> {
    pub column: ColumnIndex,
    pub text: LiteralSpelling<'a>,
}

/// An absent authored symbol spelling can rule out every row without building an index.
/// Other unsupported predicates still need ordinary row evaluation.
pub(in crate::application::file_dag::derived::owner::selection) enum Discriminator {
    Unsupported,
    Impossible,
    Indexable(ColumnIndex, Value),
}
