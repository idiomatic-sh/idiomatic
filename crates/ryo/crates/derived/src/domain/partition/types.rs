//! The types of the `partition` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether two rows have the same key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyRelation {
    /// Every key cell of the two rows is equal.
    Same,
    /// Some key cell of the two rows differs.
    Different,
}
