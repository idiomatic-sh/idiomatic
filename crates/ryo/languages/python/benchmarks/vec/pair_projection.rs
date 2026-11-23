//! One projected write per file and relation from the existing column provider.
use ryo::languages::{
    self, CellKind, CellWord, FileWords, FilledColumn, FilledColumns, Position, Relation,
    node_table,
};
use ryo_python::fixtures::columns::{Ask, ColumnFile, ColumnMemo, FactTable, Layout, TextIds};
use ryo_relations::domain::{relations::service as catalog, terms::types::NameText};

struct Words<'a>(&'a mut FileWords, &'a mut bool);
impl TextIds for Words<'_> {
    fn id(&mut self, bytes: &[u8]) -> u32 {
        if let Ok(text) = std::str::from_utf8(bytes)
            && let Ok(id) = u32::try_from(languages::facts::intern(self.0, NameText(text)).0)
            && id != u32::MAX
        {
            return id;
        }
        *self.1 = true;
        u32::MAX
    }
}
#[derive(Default)]
pub(super) struct Projection {
    table: FactTable,
    unrepresentable_word: bool,
    words: ColumnMemo,
}
impl Projection {
    pub(super) fn fill(
        &mut self,
        file: &mut ColumnFile<'_, '_>,
        number: usize,
        relation: Relation,
        key: usize,
        mask: u64,
        words: &mut FileWords,
        out: &mut FilledColumns,
    ) -> bool {
        if self.unrepresentable_word {
            return false;
        }
        let t = &mut self.table;
        file.column_with(
            &Ask {
                relation,
                key,
                exists: false,
                columns: mask,
            },
            &mut Words(words, &mut self.unrepresentable_word),
            t,
            &mut self.words,
        );
        if !t.answered || self.unrepresentable_word {
            return false;
        }
        out.0.clear();
        for (col, &kind) in catalog::kinds(relation).iter().enumerate() {
            let mut result = Vec::new();
            if mask & (1 << col) != 0 {
                let keys = t.keys();
                result.reserve(keys.len());
                if col == key {
                    result.extend(keys.iter().map(|&v| CellWord(u64::from(v))));
                } else if let Some(c) = t.columns.get(col).filter(|v| !v.is_empty()) {
                    let widen = |v| {
                        CellWord(if v == u32::MAX {
                            super::finish::NULL
                        } else {
                            u64::from(v)
                        })
                    };
                    if t.layout == Layout::PerNode {
                        result.extend(keys.iter().map(|k| widen(c[*k as usize])));
                    } else {
                        result.extend(c.iter().copied().map(widen));
                    }
                } else if kind == CellKind::File {
                    result.resize(keys.len(), CellWord(number as u64));
                } else if kind == CellKind::Span {
                    result.extend(keys.iter().map(|k| {
                        let span = node_table::span_of(file.table, Position(*k));
                        CellWord(u64::from(span.start.0) << 32 | u64::from(span.end.0))
                    }));
                } else if !keys.is_empty() {
                    return false;
                }
            }
            out.0.push(FilledColumn(result));
        }
        true
    }
}
