//! The types of the `functions` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Text a function made for one row, or the text of one of its arguments: owned, as no row holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MadeText(pub String);
