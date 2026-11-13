//! The types of the `facts` port. Plain data, public fields and derives.

use ryo_relations::domain::relations::types::{KindBits, Value};

/// Which elements a walk visits.
#[derive(Clone, Copy, Debug)]
pub enum Kinds<'a> {
    /// Every element of the file's own tree.
    Every,
    /// Only the elements of these kinds.
    Only(&'a [KindBits]),
    /// Every element of the file's own tree, then every element the language added for reading (what `source-location` covers).
    Expanded,
}

/// The `syntax-tree` row of one element: its id, kind, text, file and span.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElementRow(pub [Value; 5]);
