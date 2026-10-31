//! The types of the `operators` collection: plain data, public fields and derives, with no constructor and no inherent impl.

/// The pattern of a capture, written with slots: `docs/{page}.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SlotPattern<'a>(pub &'a str);

/// The whole text a capture reads, which the ranges of its slots are of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WholePath<'a>(pub &'a str);

/// The end of the text a capture reads that the rest of the pattern is still to match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PathTail<'a>(pub &'a str);

/// An ordered matching key keeps full typed identity; textual cells compare their contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum OrderedKey<'a> {
    /// Text from either a source symbol or a retained worker arena.
    Text(crate::domain::table_texts::types::TextView<'a>),
    /// Exact node, span, file, number or flag identity.
    Value(ryo_language::domain::relations::types::Value),
}

/// One input row indexed for deterministic matching within its equality group.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OrderedRow<'a> {
    /// Grouping keys in declared order.
    pub keys: Vec<OrderedKey<'a>>,
    /// Position within this key group.
    pub order: ryo_language::domain::relations::types::Integer,
    /// Original row retained for the explicit output projection.
    pub row: crate::domain::table_cells::types::RowIndex,
}
