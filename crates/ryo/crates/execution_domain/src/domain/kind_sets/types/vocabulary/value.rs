//! Prepared immutable kind masks; no provider or source lifetime is retained.

use crate::domain::kind_sets::types::{ComparisonOrdinal, ListAddress};
use crate::domain::table_scratch::types::KindSet;
use ryo_values::domain::descriptions::types::KindWord;
use ryo_values::domain::plan_parts::types::LiteralText;

/// Exact ordered kind names of one prepared vocabulary and its finite literal masks.
/// These values own no file source, symbols, rows, or provider references.
#[derive(Debug)]
pub struct PreparedVocabulary {
    /// Exact names in kind-number order, including the unnamed sentinel.
    pub(crate) names: Vec<Option<KindWord>>,
    /// Lazy immutable masks for the request's literal comparisons only.
    pub(crate) masks:
        std::sync::Mutex<hashbrown::HashMap<PreparedKindTest, std::sync::Arc<KindSet>>>,
    /// Metadata-lock acquisitions during setup, absent from production.
    #[cfg(test)]
    pub(crate) lookups: std::sync::atomic::AtomicUsize,
    /// Vocabulary scans to construct literal masks, absent from production.
    #[cfg(test)]
    pub(crate) builds: std::sync::atomic::AtomicUsize,
}

/// A literal comparison's identity within the prepared request.
#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct PreparedKindTest {
    /// Stable address of the request-owned comparison list; never dereferenced.
    pub comparisons: ListAddress,
    /// Comparison ordinal within that list.
    pub ordinal: ComparisonOrdinal,
    /// Exact comparison operation.
    pub test: ryo_values::domain::plan_parts::types::Test,
    /// Exact literal text, independent of symbol numbers and text hashes.
    pub text: LiteralText,
}
