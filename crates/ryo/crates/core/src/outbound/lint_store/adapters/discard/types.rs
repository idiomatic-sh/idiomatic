//! The types of the discard store adapter: plain data, with no constructor and no inherent impl.

/// The store a run that keeps nothing reads: nothing.
#[derive(Debug)]
pub(crate) struct DiscardRead;
