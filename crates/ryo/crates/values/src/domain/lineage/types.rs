//! The types of the `lineage` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Where the rows of a step come from.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::lineage::types::{ColumnAnchor, Lineage};
/// let one = Lineage::OneFile([ColumnAnchor::InFile].into());
/// assert_ne!(one, Lineage::Mixed);
/// assert_eq!(Lineage::default(), Lineage::Mixed);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Lineage {
    /// A row may be made of the rows of more than one file, so no column says which file it comes from.
    #[default]
    Mixed,
    /// Each row is made of the rows of one file: for each column, whether its file or node lies in that file.
    OneFile(Box<[ColumnAnchor]>),
}

/// Whether a column's file or node lies in the file its row comes from.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::lineage::{service::anchor_of, types::ColumnAnchor};
/// assert_eq!(anchor_of(CellKind::File), ColumnAnchor::InFile);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnAnchor {
    /// The column holds the file the row comes from, or a node of it.
    InFile,
    /// The column holds no file or node, or one that may lie in another file.
    Elsewhere,
}

/// Whether the keys of two operands pair a column of each that lies in its row's file.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// use ryo_values::domain::lineage::service::{one_file, pairing};
/// use ryo_values::domain::lineage::types::KeyPairing;
/// let rows = one_file(&[CellKind::Node]);
/// assert_eq!(pairing(&rows, &[ColumnIndex(0)], &rows, &[ColumnIndex(0)], &[]), KeyPairing::SameFile);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyPairing {
    /// Some key pairs a column of each operand that lies in its row's file: the rows the keys match come from one file.
    SameFile,
    /// No key proves that the rows it matches come from one file.
    Unproven,
}

/// Whether a value of a kind names the file it lies in, so the file can be read from the value itself.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::CellKind;
/// use ryo_values::domain::lineage::{service::naming_of, types::FileNaming};
/// assert_eq!(naming_of(CellKind::Node), FileNaming::Named);
/// assert_eq!(naming_of(CellKind::Span), FileNaming::Unnamed);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileNaming {
    /// The value is a file, or a node, whose number holds its file.
    Named,
    /// The value holds no file: a span is a place in its row's file but does not say which.
    Unnamed,
}
