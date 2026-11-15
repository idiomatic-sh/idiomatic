//! Prepared immutable kind masks; no provider or source lifetime is retained.

use ryo_operators::domain::table_scratch::types::KindSet;
use ryo_values::domain::descriptions::types::KindWord;
use ryo_values::domain::plan_parts::types::LiteralText;

/// Exact ordered kind names of one prepared vocabulary and its finite literal masks.
/// These values own no file source, symbols, rows, or provider references.
#[derive(Debug)]
pub struct PreparedVocabulary {
    /// Exact names in kind-number order, including the unnamed sentinel.
    pub(crate) names: Vec<Option<KindWord>>,
}
