//! The closed answers of the `literal_properties` concept.

/// A known primitive category and the three explicitly limited predicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Properties {
    /// The closed primitive category.
    pub kind: &'static str,
    /// Whether a numeric parser value is exactly zero.
    pub zero: bool,
    /// Whether a decoded string or bytes parser value is empty.
    pub empty: bool,
    /// Whether the parsed boolean is `True`.
    pub boolean: bool,
}

/// A constant that is not a primitive occurrence, has a complete primitive payload, or lacks one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiteralClassification {
    /// Not a primitive literal row, including synthetic interpolation fragments.
    NotPrimitive,
    /// A complete parser-derived result.
    Known(Properties),
    /// A primitive node exists but lacks required parser payload.
    MissingPayload,
}
