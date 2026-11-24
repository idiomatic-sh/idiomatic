//! Complete catalog rows produced by this file's prefix. The provider retains
//! their original cell words and projects them only when the file asks.
use ryo::languages::{FileId, FilledColumns, Relation};
use ryo_relations::domain::relations::service::number_of;

#[derive(Default)]
pub(super) struct Completed {
    pub(super) file: Option<FileId>,
    tables: Vec<Option<FilledColumns>>,
    pub(super) empty: Vec<Relation>,
    pub(super) absent_symbols: Vec<u32>,
    pub(super) decisions: Vec<Option<bool>>,
}

impl Completed {
    pub(super) fn begin(&mut self, file: FileId) {
        self.absent_symbols.clear();
        self.decisions.clear();
        self.tables.clear();
        self.empty.clear();
        self.file = Some(file);
    }

    pub(super) fn take(&mut self, relation: Relation) -> Option<FilledColumns> {
        self.tables.get_mut(number_of(relation).0)?.take()
    }

    pub(super) fn clear(&mut self) {
        self.absent_symbols.clear();
        self.decisions.clear();
        self.tables.clear();
        self.empty.clear();
        self.file = None;
    }

    pub(super) fn table(&mut self, relation: Relation, rows: FilledColumns) {
        if rows.0.first().is_some_and(|column| column.0.is_empty()) {
            self.empty.push(relation);
        }
        let slot = number_of(relation).0;
        if self.tables.len() <= slot {
            self.tables.resize_with(slot + 1, || None);
        }
        self.tables[slot] = Some(rows);
    }

}
