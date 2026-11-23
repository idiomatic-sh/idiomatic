//! Complete catalog rows produced by this file's prefix. The provider retains
//! their original cell words and projects them only when the file asks.
use ryo::languages::{FileId, FilledColumns, Relation};
use ryo_relations::domain::relations::service::number_of;

#[derive(Default)]
pub(super) struct Completed {
    pub(super) file: Option<FileId>,
    tables: Vec<Option<FilledColumns>>,
    pub(super) empty: Vec<Relation>,
    pub(super) empty_drivers: Vec<usize>,
}

impl Completed {
    pub(super) fn begin(&mut self, file: FileId) {
        self.tables.clear();
        self.empty_drivers.clear();
        self.empty.clear();
        self.file = Some(file);
    }

    pub(super) fn take(&mut self, relation: Relation) -> Option<FilledColumns> {
        self.tables.get_mut(number_of(relation).0)?.take()
    }

    pub(super) fn clear(&mut self) {
        self.tables.clear();
        self.empty_drivers.clear();
        self.empty.clear();
        self.file = None;
    }
}

pub(super) struct FileWitness<'a, 'p> {
    pub(super) completed: &'a mut Completed,
    pub(super) exec: &'a mut super::finish::Exec<'p>,
    pub(super) run: &'a super::finish::Run,
}

impl super::pair_checks::WitnessResults for FileWitness<'_, '_> {
    fn table(&mut self, relation: Relation, rows: FilledColumns) {
        if rows.0.first().is_some_and(|column| column.0.is_empty()) {
            self.completed.empty.push(relation);
        }
        let slot = number_of(relation).0;
        if self.completed.tables.len() <= slot {
            self.completed.tables.resize_with(slot + 1, || None);
        }
        self.completed.tables[slot] = Some(rows);
    }

    fn empty_driver(&mut self, step: usize) {
        self.completed.empty_drivers.push(step);
    }

    fn indexed_symbols(&mut self, relation: &str, column: usize, present: &dyn Fn(&str) -> Option<bool>) {
        self.exec.indexed_symbols(self.run, relation, column, present);
    }
}
