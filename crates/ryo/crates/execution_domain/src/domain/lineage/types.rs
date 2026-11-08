//! The types of the `lineage` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Where the rows of a step come from.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) enum Lineage {
    /// A row may be made of the rows of more than one file, so no column says which file it comes from.
    #[default]
    Mixed,
    /// Each row is made of the rows of one file: for each column, whether its file or node lies in that file.
    OneFile(Box<[ColumnAnchor]>),
}

/// Whether a column's file or node lies in the file its row comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColumnAnchor {
    /// The column holds the file the row comes from, or a node of it.
    InFile,
    /// The column holds no file or node, or one that may lie in another file.
    Elsewhere,
}

/// Whether the keys of two operands pair a column of each that lies in its row's file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyPairing {
    /// Some key pairs a column of each operand that lies in its row's file: the rows the keys match come from one file.
    SameFile,
    /// No key proves that the rows it matches come from one file.
    Unproven,
}
