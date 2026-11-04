//! Borrowed representation data; admission and retained ownership live in the engine.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::TreeRows;

use ryo_relations::domain::relations::types::{CellKind, ColumnsView, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RepresentationKind {
    Stored,
    Selected,
    Tree,
    Computed,
    Compact,
    Positioned,
}
impl RepresentationKind {
    pub(crate) const ALL: &'static [Self] = &[
        Self::Stored,
        Self::Selected,
        Self::Tree,
        Self::Computed,
        Self::Compact,
        Self::Positioned,
    ];
}
#[derive(Clone, Copy)]
pub(crate) enum RepresentationInput<'a> {
    Stored(ColumnsView<'a>),
    Selected {
        source: ColumnsView<'a>,
        positions: &'a [u32],
        columns: &'a [ColumnIndex],
        kinds: &'a [CellKind],
    },
    Tree(TreeRows<'a>),
    Positioned(crate::domain::table_model::types::PositionedRows<'a>),
    Computed(&'a Table),
    Compact(&'a crate::domain::table_words::types::CompactTable),
}
pub(crate) type BulkAccess<'a> = RepresentationInput<'a>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysicalShape {
    Complete,
    ReadablePhysicalFallback,
}
pub(crate) struct RelationContract<'a> {
    pub input: RepresentationInput<'a>,
    pub kind: RepresentationKind,
    pub kinds: &'a [CellKind],
    pub rows: usize,
    /// Captured metadata refusals are interpreted only at the existing demand gate.
    pub metadata: Result<(), OperatingError>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellRead {
    Value(Value),
    MissingPhysical,
}
pub(crate) enum RowRead<'a> {
    Borrowed(&'a [Value]),
    Decoded(&'a [Value]),
    MissingPhysical,
}

use ryo_values::domain::rows::types::Table;
