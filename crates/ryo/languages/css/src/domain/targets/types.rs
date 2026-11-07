//! The types of the `targets` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// A url as the library decoded it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Url<'a>(pub &'a str);
