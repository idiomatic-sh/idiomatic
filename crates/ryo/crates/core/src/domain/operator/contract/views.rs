//! Read-only physical inputs expose no owned cache or retained selection handle.
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::types::KeyIndex;
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(crate) use crate::domain::table_model::collections::representations::types::RepresentationInput as BorrowedRows;

/// Exact typed key lookup prepared by the engine; primitives only probe it.
#[derive(Clone, Copy)]
pub(crate) struct IndexedRows<'a> {
    pub rows: BorrowedRows<'a>,
    pub columns: &'a [ColumnIndex],
    pub index: &'a KeyIndex,
}
impl IndexedRows<'_> {
    pub(crate) fn contains(&self, key: &[Value]) -> bool {
        self.matching(key).next().is_some()
    }
    pub(crate) fn matching<'a>(&'a self, key: &'a [Value]) -> MatchingRows<'a> {
        let hash = crate::domain::table_model::service::hash_key(key.iter().map(Some));
        let bucket =
            usize::try_from(hash.0).unwrap_or(0) & self.index.heads.len().saturating_sub(1);
        MatchingRows {
            view: self,
            key,
            hash,
            at: self
                .index
                .heads
                .get(bucket)
                .copied()
                .unwrap_or(crate::domain::table_model::types::ChainLink(0)),
        }
    }
}
pub(crate) struct MatchingRows<'a> {
    view: &'a IndexedRows<'a>,
    key: &'a [Value],
    hash: crate::domain::table_model::types::KeyCellsHash,
    at: crate::domain::table_model::types::ChainLink,
}
impl Iterator for MatchingRows<'_> {
    type Item = RowIndex;
    fn next(&mut self) -> Option<RowIndex> {
        use crate::domain::table_model::types::ChainLink;
        while self.at.0 != 0 {
            let position = usize::try_from(self.at.0 - 1).unwrap_or(usize::MAX);
            self.at = self
                .view
                .index
                .next
                .get(position)
                .copied()
                .unwrap_or(ChainLink(0));
            let row = self.view.index.rows.get(position).copied()?;
            if self.view.index.hashes.get(position) == Some(&self.hash)
                && self
                    .view
                    .columns
                    .iter()
                    .zip(self.key)
                    .all(|(column, expected)| self.view.rows.cell(row, *column) == Some(*expected))
            {
                return Some(row);
            }
        }
        None
    }
}
