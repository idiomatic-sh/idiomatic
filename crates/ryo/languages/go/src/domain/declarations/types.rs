//! The types of the `declarations` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether the fields of a list of parameters were found together, so that the declarations in it make no field of their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Grouping {
    /// The list names some of its parameters: its fields were found, a group of names that share a type being one.
    Grouped,
    /// The list names none: each declaration in it is a field.
    Ungrouped,
}
