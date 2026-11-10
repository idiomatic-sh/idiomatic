//! The types of the `allocation` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The address that identifies one allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AllocationKey(pub usize);

/// The number of elements an allocation can hold without growing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElementCapacity(pub usize);

/// A number of bytes an allocation occupies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct AllocationBytes(pub usize);
