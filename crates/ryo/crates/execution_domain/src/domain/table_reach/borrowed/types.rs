//! The types of the `borrowed` concept: plain data, public fields and derives, with no constructor and no inherent impl.

#[cfg(feature = "diagnostic-row-progress")]
use crate::domain::operating::types::ProgressObserver;
use crate::domain::table_model::types::RepresentationInput;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

/// Exact typed key lookup prepared by the engine; primitives only probe it.
#[derive(Clone, Copy)]
pub struct IndexedRows<'a> {
    /// The observer the probes report their work to.
    #[cfg(feature = "diagnostic-row-progress")]
    pub progress: &'a dyn ProgressObserver,
    /// The rows indexed.
    pub rows: RepresentationInput<'a>,
    /// The key columns.
    pub columns: &'a [ColumnIndex],
    /// The key index over the rows.
    pub index: &'a KeyIndex,
}

/// Physical hash-chain position, independent of the caller-owned key buffer.
pub(crate) struct KeyProbe {
    pub(crate) hash: ryo_derived::domain::key::types::KeyCellsHash,
    pub(crate) at: ryo_derived::domain::key::types::ChainLink,
}

pub(crate) struct MatchingRows<'a> {
    pub(crate) view: &'a IndexedRows<'a>,
    pub(crate) key: &'a [Value],
    pub(crate) probe: KeyProbe,
}

impl Iterator for MatchingRows<'_> {
    type Item = RowIndex;
    fn next(&mut self) -> Option<RowIndex> {
        crate::domain::table_reach::borrowed::service::next_match(
            self.view,
            self.key,
            &mut self.probe,
        )
    }
}
