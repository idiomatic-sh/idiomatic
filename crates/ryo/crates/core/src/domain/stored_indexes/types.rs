//! Private generation-local cache data, with no retained source or provider references.
use crate::domain::table_model::types::KeyIndex;
use ryo_language::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

/// One exact immutable stored population and its ordered typed key schema.
#[derive(Debug)]
pub(crate) struct Entry {
    /// Logical relation, never a pointer identity.
    pub relation: Relation,
    /// Key positions in argument order, including repeats.
    pub columns: Vec<ColumnIndex>,
    /// Static schema compared by contents.
    pub kinds: &'static [CellKind],
    /// Original row population of the frozen relation.
    pub rows: usize,
    /// Immutable index; callers own a handle before releasing the mutable cache borrow.
    pub index: Arc<KeyIndex>,
}

/// A file-local owner: dropping it releases all admitted allocations without eviction.
#[derive(Debug)]
pub(crate) struct FileIndexes {
    /// Active relations whose physical read reported no partiality, of any missing kind.
    pub eligible: Vec<Relation>,
    /// Admitted indexes and their exact keys; linear lookup is bounded by the byte budget.
    pub entries: Vec<Entry>,
    /// Accounted actual capacities and conservative allocation metadata.
    pub bytes: usize,
    /// Maximum retained cache bytes for this file and worker.
    pub limit: usize,
    /// Physical stored-key builds, including invocation-local admission refusals.
    pub builds: u64,
    /// Successful immutable index reuse.
    pub reuses: u64,
    /// Eligible requests whose index could not be retained within the bound.
    pub refusals: u64,
}

/// The index prepared for one join; no mutable cache borrow escapes acquisition.
#[derive(Debug)]
pub(crate) struct Acquired {
    /// A shared index, or the already-built index in the caller's local scratch.
    pub shared: Option<Arc<KeyIndex>>,
    /// Whether this acquisition physically built an index.
    pub built: bool,
    /// Whether the physical relation was eligible for sharing.
    pub eligible: bool,
}
