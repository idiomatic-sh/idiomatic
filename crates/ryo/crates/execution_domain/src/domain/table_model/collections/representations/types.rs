//! Borrowed representation data; admission and retained ownership live in the engine.

use ryo_relations::domain::relations::types::Value;
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
