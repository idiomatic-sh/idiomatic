//! The types of the `operation_schema` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::table_cells::types::TreeForm;
use ryo_language::domain::relations::types::CellKind;

pub(crate) mod bindings;
pub(crate) mod traversal;

/// Ordered input column kinds and representation, with no payload or source access.
#[derive(Clone, Copy, Debug)]
pub(crate) struct InputSchema<'a> {
    /// Column kinds, preserving order, repetition and zero-width inputs.
    pub kinds: &'a [CellKind],
    /// Native syntax representation, absent for resident and computed tables.
    pub tree: Option<TreeForm>,
}
