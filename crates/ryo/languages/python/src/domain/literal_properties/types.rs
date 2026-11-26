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
