//! The types of the `answering` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The heading of a column of a row the lint call answered, as a reader names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Heading<'a>(pub &'a str);

/// The value of a column of a row, owned: empty when the row does not have it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Owned(pub Box<str>);
