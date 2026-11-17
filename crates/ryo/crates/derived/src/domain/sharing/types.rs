//! The types of the `sharing` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use std::sync::Arc;

/// Immutable shared membership marks over a dense element space.
#[derive(Clone, Debug)]
pub struct SharedMarks(pub Arc<Vec<bool>>);
