//! The types of the `index` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Owned immutable view handles stay in application; primitives borrow only the index.
/// Whether an index over an empty input is skipped or still built.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmptyInput {
    Skip,
    Keep,
}
