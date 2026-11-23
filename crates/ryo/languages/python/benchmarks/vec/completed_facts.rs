//! Complete catalog rows produced by this file's prefix. The provider retains
//! their original cell words and projects them only when the file asks.
use ryo::languages::{FileId, FilledColumns, Relation};
use ryo_relations::domain::relations::service::number_of;

#[derive(Default)]
pub(super) struct Completed {
    pub(super) file: Option<FileId>,
    tables: Vec<Option<FilledColumns>>,
}

impl Completed {
    pub(super) fn begin(&mut self, file: FileId) {
        self.tables.clear();
        self.file = Some(file);
    }

    pub(super) fn insert(&mut self, relation: Relation, rows: FilledColumns) {
        let slot = number_of(relation).0;
        if self.tables.len() <= slot {
            self.tables.resize_with(slot + 1, || None);
        }
        self.tables[slot] = Some(rows);
    }

    pub(super) fn take(&mut self, relation: Relation) -> Option<FilledColumns> {
        self.tables.get_mut(number_of(relation).0)?.take()
    }

    pub(super) fn clear(&mut self) {
        self.tables.clear();
        self.file = None;
    }
}
