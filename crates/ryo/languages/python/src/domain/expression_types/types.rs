//! Retained data for the lazy basic expression-type reader.
#[derive(Clone, Debug, PartialEq, Eq)]
/// One expression's basic inferred type, with union members in deterministic traversal order.
pub(crate) enum Resolution {
    /// A single known basic Python type.
    Atom(BasicType),
    /// More than one known type after simple subtype reduction.
    Union(Vec<BasicType>),
    /// The expression's type is not determined by its syntax alone.
    Unknown,
    /// The supported operation has incompatible known operand types.
    TypeError,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// A per-element bottom-up index, built only when the relation is requested.
pub(crate) struct ExpressionTypeIndex {
    /// Classification by parser position; only expression nodes receive entries.
    pub entries: Vec<Option<Resolution>>,
    /// Whether the current parse was represented completely.
    pub state: IndexState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Whether a retained expression-type index belongs to the current source.
pub(crate) enum IndexState {
    #[default]
    /// No relation reader has requested a build since the last source reset.
    Fresh,
    /// Every expression and required literal payload was represented.
    Complete,
    /// Syntax or a required parser payload was incomplete.
    Partial,
}

/// Closed basic types supplied by the Python expression classifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BasicType {
    /// The `boolean` classification.
    Boolean,
    /// The `integer` classification.
    Integer,
    /// The `float` classification.
    Float,
    /// The `complex` classification.
    Complex,
    /// The `string` classification.
    String,
    /// The `bytes` classification.
    Bytes,
    /// The `list` classification.
    List,
    /// The `tuple` classification.
    Tuple,
    /// The `set` classification.
    Set,
    /// The `dict` classification.
    Dict,
    /// The `generator` classification.
    Generator,
    /// The `null` classification.
    Null,
    /// The `ellipsis` classification.
    Ellipsis,
    /// The `unknown` classification.
    Unrecognized,
}
