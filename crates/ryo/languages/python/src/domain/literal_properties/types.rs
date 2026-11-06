//! The closed answers of the `literal_properties` concept.
use ryo::languages::{Flag, NameText};

/// A known primitive category and the three explicitly limited predicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Properties {
    /// The closed primitive category.
    pub kind: NameText<'static>,
    /// Whether a numeric parser value is exactly zero.
    pub zero: Flag,
    /// Whether a decoded string or bytes parser value is empty.
    pub empty: Flag,
    /// Whether the parsed boolean is `True`.
    pub boolean: Flag,
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

/// Whether the parser's `Constant` node is a synthetic piece of an interpolated string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Interpolated(pub bool);
