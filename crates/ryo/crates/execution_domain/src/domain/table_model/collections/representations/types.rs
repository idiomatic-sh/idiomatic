//! Borrowed representation data; admission and retained ownership live in the engine.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::{RepresentationInput, RepresentationKind};

use ryo_relations::domain::relations::types::{CellKind, Value};
/// What a representation declares of its rows: the input, its kind, its cell kinds and its row count.
pub struct RelationContract<'a> {
    /// The rows' representation.
    pub input: RepresentationInput<'a>,
    /// The kind of the representation.
    pub kind: RepresentationKind,
    /// The cell kinds of its columns.
    pub kinds: &'a [CellKind],
    /// How many rows it holds.
    pub rows: RowCount,
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

use ryo_values::domain::rows::types::RowCount;
