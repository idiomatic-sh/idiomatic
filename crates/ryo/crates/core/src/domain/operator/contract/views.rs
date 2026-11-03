//! Read-only physical inputs expose no owned cache or retained selection handle.

use ryo_derived::domain::contract::types::KeyIndex;
use ryo_relations::domain::relations::types::Value;
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
    pub(crate) fn probe(&self, key: &[Value]) -> KeyProbe {
        let hash = ryo_derived::domain::contract::key::hash_key(key.iter().map(Some));
        let bucket =
            usize::try_from(hash.0).unwrap_or(0) & self.index.heads.len().saturating_sub(1);
        KeyProbe {
            hash,
            at: self
                .index
                .heads
                .get(bucket)
                .copied()
                .unwrap_or(ryo_derived::domain::contract::types::ChainLink(0)),
        }
    }
    pub(crate) fn next_match(&self, key: &[Value], probe: &mut KeyProbe) -> Option<RowIndex> {
        use ryo_derived::domain::contract::types::ChainLink;
        while probe.at.0 != 0 {
            let position = usize::try_from(probe.at.0 - 1).unwrap_or(usize::MAX);
            probe.at = self
                .index
                .next
                .get(position)
                .copied()
                .unwrap_or(ChainLink(0));
            let row = self.index.rows.get(position).copied()?;
            if self.index.hashes.get(position) == Some(&probe.hash)
                && self
                    .columns
                    .iter()
                    .zip(key)
                    .all(|(column, expected)| self.rows.cell(row, *column) == Some(*expected))
            {
                return Some(row);
            }
        }
        None
    }
    pub(crate) fn matching<'a>(&'a self, key: &'a [Value]) -> MatchingRows<'a> {
        MatchingRows {
            view: self,
            key,
            probe: self.probe(key),
        }
    }
}
/// Physical hash-chain position, independent of the caller-owned key buffer.
pub(crate) struct KeyProbe {
    hash: ryo_derived::domain::contract::types::KeyCellsHash,
    at: ryo_derived::domain::contract::types::ChainLink,
}
pub(crate) struct MatchingRows<'a> {
    view: &'a IndexedRows<'a>,
    key: &'a [Value],
    probe: KeyProbe,
}
impl Iterator for MatchingRows<'_> {
    type Item = RowIndex;
    fn next(&mut self) -> Option<RowIndex> {
        self.view.next_match(self.key, &mut self.probe)
    }
}

use ryo_values::domain::rows::types::RowIndex;
