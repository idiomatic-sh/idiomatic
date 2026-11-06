//! The braceformat adapter's retained part and selector types.

#[derive(Clone, Debug)]
/// A replacement field retained by the brace grammar.
pub(super) enum Part {
    /// A field root and its optional nested formatting specification.
    Field {
        /// The field's selector and conversion.
        name: String,
        /// The text following the field's colon.
        spec: Option<String>,
        conversion: Option<char>,
    },
}

#[derive(Clone, Debug)]
/// The root selector before attribute and subscript operations.
pub(super) enum FieldType {
    /// An omitted selector using implicit positional numbering.
    Automatic,
    /// A parsed positional index with its original spelling.
    Index(usize, String),
    /// A keyword selector.
    Keyword(String),
}
