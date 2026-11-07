//! The types of the `kind_sets` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::table_scratch::types::{ComparisonNumbers, KindSet, KindSetPlace, KindTest};

/// The names, sets and tests of kinds a filter works with, taken out of the scratch while it runs.
pub(crate) struct KindScratch {
    /// For each test, the sets by the text the kind is tested against.
    pub names: hashbrown::HashMap<KindTest, KindSetPlace>,
    /// The sets of kinds made so far.
    pub sets: Vec<std::sync::Arc<KindSet>>,
    /// For each comparison that names a kind, the place of its set.
    pub tests: ComparisonNumbers,
}

pub(crate) mod vocabulary;

/// The stable address of a request-owned list of comparisons, never dereferenced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ListAddress(pub usize);

/// The place of a comparison within its list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ComparisonOrdinal(pub usize);
