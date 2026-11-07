//! The types of the `native` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Which native table a lookup reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeForm {
    /// The element table.
    Elements,
    /// The edge table.
    Edges,
}

/// The native lookup a static shape reduces to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeFinding {
    /// A lookup by node: the left column and the native key column.
    Node(ColumnIndex, ColumnIndex),
    /// A lookup by file and stretch: the left file column and the left stretch column.
    Stretch(ColumnIndex, ColumnIndex),
    /// A lookup by span: the left span column.
    Span(ColumnIndex),
}
