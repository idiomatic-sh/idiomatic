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

/// Whether one known type is included in another under Python numeric coercion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TypeInclusion {
    Included,
    Separate,
}
