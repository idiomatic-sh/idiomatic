//! Content-only results that may survive one source revision.
use std::collections::HashMap;
use super::constants;
/// One result and the capacity of its owned child buffers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry<V> {
    /// The application-owned grammar result.
    pub value: V,
    /// Key and result child capacity in bytes.
    pub bytes: usize,
}
/// A byte-budgeted, single-previous-source memo, separate from active file outputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GrammarMemo<V> {
    /// Exact input text; a memo belongs to one dialect and parser type.
    pub entries: HashMap<String, Entry<V>>,
    /// Sum of owned entry capacities.
    pub bytes: usize,
    /// Static parser type identity, including the caller's grammar marker.
    pub grammar: (&'static str, &'static str),
    /// Reusable input assembly storage, bounded independently of active file data.
    pub scratch: String,
}
impl<V> Default for GrammarMemo<V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            bytes: 0,
            grammar: (constants::EMPTY, constants::EMPTY),
            scratch: String::new(),
        }
    }
}
