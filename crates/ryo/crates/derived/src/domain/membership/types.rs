//! The types of the `membership` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::positions::types::DenseMarks;
use crate::domain::sharing::types::SharedMarks;
use ryo_relations::domain::relations::types::{FileId, Value};

/// The number of elements of a native element space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkCount(pub usize);

/// The space the members of a column are marked in.
pub enum MarkSpace<'a> {
    /// The member values themselves.
    Values,
    /// One mark for each value of a dense vertex list.
    Dense(&'a [Value]),
    /// One mark for each element of a native file.
    Native {
        /// The file whose elements are marked.
        file: FileId,
        /// The number of elements of the file.
        elements: MarkCount,
    },
}

/// What a membership build retains.
#[derive(Debug)]
pub enum DerivedMembership {
    /// The member values.
    Values(hashbrown::HashSet<Value>),
    /// Shared marks over a dense element space.
    Dense(SharedMarks),
}

/// Immutable borrowed membership never lends an owned sub-allocation handle to a primitive.
#[derive(Clone, Copy)]
pub enum MembershipView<'a> {
    /// The member values.
    Values(&'a hashbrown::HashSet<Value>),
    /// The marks of a dense space.
    Dense(DenseMarks<'a>),
}
