//! The types of the `allocation` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The number of elements an allocation can hold without growing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElementCapacity(pub usize);
