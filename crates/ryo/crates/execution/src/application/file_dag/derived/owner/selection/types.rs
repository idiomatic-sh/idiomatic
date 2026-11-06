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

pub(in crate::application::file_dag::derived::owner) enum CandidateRows {
    Owned(Vec<u32>),
    Shared {
        rows: Arc<Vec<u32>>,
        _resource: crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    },
}

impl CandidateRows {
    pub(in crate::application::file_dag::derived::owner) fn as_slice(&self) -> &[u32] {
        match self {
            Self::Owned(rows) => rows,
            Self::Shared { rows, .. } => rows.as_ref().as_slice(),
        }
    }
}

/// An absent authored symbol spelling can rule out every row without building an index.
/// Other unsupported predicates still need ordinary row evaluation.
pub(in crate::application::file_dag::derived::owner::selection) enum Discriminator {
    Unsupported,
    Impossible,
    Indexable(ColumnIndex, Value),
}
