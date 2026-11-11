//! The braceformat adapter's retained part and selector types.
use crate::domain::brace_format::types::{BraceArgumentIndex, BraceSelectorText, BraceSpecText};
use crate::domain::format_syntax::types::FormatCharacter;

#[derive(Clone, Debug)]
/// A replacement field retained by the brace grammar.
pub(super) enum Part {
    /// A field root and its optional nested formatting specification.
    Field {
        /// The field's selector and conversion.
        name: BraceSelectorText,
        /// The text following the field's colon.
        spec: Option<BraceSpecText>,
        /// The character after the field's exclamation mark.
        conversion: Option<FormatCharacter>,
    },
}

#[derive(Clone, Debug)]
/// The root selector before attribute and subscript operations.
pub(super) enum FieldType {
    /// An omitted selector using implicit positional numbering.
    Automatic,
    /// A parsed positional index with its original spelling.
    Index(BraceArgumentIndex, BraceSelectorText),
    /// A keyword selector.
    Keyword(BraceSelectorText),
}
