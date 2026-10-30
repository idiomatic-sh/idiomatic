//! The types of the memory adapter: plain data, public fields and derives, with no constructor and no inherent impl.

/// What a refusal of the adapter says.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RefusalText<'a>(pub &'a str);
