//! The types of the `contributor_rows` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The operands, by their place, whose absence of rows an operator trusts: the bit of an operand is `1 << place`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AbsenceMask(pub u8);
