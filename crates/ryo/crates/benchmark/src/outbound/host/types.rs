//! What the host port observes beside the shared facts.

/// The processors this process may run on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessorCount(pub std::num::NonZeroUsize);
