//! An index for one demanded symbol column. The provider writes query symbol
//! numbers directly. A zero denotes a value outside the query vocabulary.
use super::pair_summary::Id;
use rustc_hash::FxHashMap as Map;
use ryo::languages::Relation;
use ryo_python::fixtures::columns::{Ask, ColumnFile, ColumnMemo, FactTable, Layout, TextIds};

struct Symbols<'a>(&'a Map<String, Id>);
impl TextIds for Symbols<'_> {
    fn id(&mut self, text: &[u8]) -> u32 {
        std::str::from_utf8(text)
            .ok()
            .and_then(|s| self.0.get(s))
            .map_or(0, |v| {
                u32::try_from(*v + 1).expect("query symbol fits in a cell")
            })
    }
}
#[derive(Default)]
pub(super) struct Columns {
    pub project_keys: bool,
    pub memo: ColumnMemo,
    table: FactTable,
}
impl Columns {
    pub fn keys(
        &mut self,
        file: &mut ColumnFile<'_, '_>,
        relation: Relation,
        key: usize,
        symbols: &Map<String, Id>,
        out: &mut Vec<u64>,
        rows: &mut usize,
    ) -> Option<bool> {
        if !self.project_keys || !ColumnFile::projects_nodes(relation, key) {
            return None;
        }
        let t = &mut self.table;
        file.column_with(
            &Ask {
                relation,
                key,
                exists: true,
                columns: 0,
            },
            &mut Symbols(symbols),
            t,
            &mut self.memo,
        );
        *rows = t.rows;
        out.extend_from_slice(&t.present);
        Some(t.answered)
    }
    pub fn read(
        &mut self,
        file: &mut ColumnFile<'_, '_>,
        relation: Relation,
        key: usize,
        column: usize,
        symbols: &Map<String, Id>,
        index: &mut Map<Id, Vec<u32>>,
        rows: &mut usize,
    ) -> Option<bool> {
        if !ColumnFile::projects_symbols(relation, key, column) {
            return None;
        }
        let table = &mut self.table;
        file.column_with(
            &Ask {
                relation,
                key,
                exists: false,
                columns: 1u64.checked_shl(column as u32)?,
            },
            &mut Symbols(symbols),
            table,
            &mut self.memo,
        );
        *rows = table.rows;
        if !table.answered {
            return Some(false);
        }
        let data = table.columns.get(column)?;
        for (row, &node) in table.keys().iter().enumerate() {
            let place = if table.layout == Layout::PerNode {
                node as usize
            } else {
                row
            };
            if let Some(id) = data[place].checked_sub(1) {
                index.entry(id as usize).or_default().push(node);
            }
        }
        Some(true)
    }
}
