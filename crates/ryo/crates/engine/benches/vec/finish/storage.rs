//! Bind the executor's native word and ordinal slices to the worker's product storage.
use super::{Names, Tys};
use ryo::languages::CellWord;
use ryo_engine::domain::{indexes::postings, made_tables::materialized};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::rows::types::RowIndex;
use std::{cell::RefCell, rc::Rc};

pub(crate) type Table = materialized::types::ResultColumns<u64, String>;
pub(crate) type Index = postings::types::PostingIndex;
#[derive(Default)]
pub(crate) struct Storage {
    tables: RefCell<materialized::types::TableBuffers<u64, String>>,
    postings: RefCell<postings::types::PostingBuffers>,
    candidates: RefCell<postings::types::PackedPositions>,
    selections: RefCell<materialized::types::SelectionBuffers<u32>>,
}
impl Storage {
    pub(crate) fn table(
        &self,
        names: impl Into<Names>,
        kinds: impl Into<Tys>,
        length: usize,
    ) -> Table {
        materialized::service::prepare(
            &mut self.tables.borrow_mut(),
            names.into(),
            kinds.into(),
            Count(length),
        )
    }
    pub(crate) fn empty(&self, names: impl Into<Names>, kinds: impl Into<Tys>) -> Table {
        self.table(names, kinds, 0)
    }
    pub(crate) fn copy(&self, table: &Table) -> Table {
        materialized::service::copy(&mut self.tables.borrow_mut(), table)
    }
    pub(crate) fn extend(&self, table: &mut Table, word: u64, length: usize) {
        materialized::service::extend(&mut self.tables.borrow_mut(), table, word, Count(length));
    }
    pub(crate) fn rc(&self, table: Table) -> Rc<Table> {
        materialized::service::share(&mut self.tables.borrow_mut(), table)
    }
    pub(crate) fn gather(
        &self,
        table: &Table,
        rows: impl ExactSizeIterator<Item = usize> + Clone,
    ) -> Table {
        materialized::service::gather_rows(&mut self.tables.borrow_mut(), table, rows.map(RowIndex))
            .expect("row ordinals belong to the source table")
    }
    pub(crate) fn subset(&self, table: &Table, index: &Index, keys: &[u64]) -> Table {
        let mut rows = self.candidates.borrow_mut();
        postings::service::collect_rows(index, keys.iter().copied().map(CellWord), &mut rows);
        self.gather(table, rows.0.iter().map(|&r| r as usize))
    }
    pub(crate) fn release(&self, table: &mut Table) {
        materialized::service::release(&mut self.tables.borrow_mut(), table);
    }
    pub(crate) fn recycle(&self, answer: Rc<Table>) {
        materialized::service::retire(&mut self.tables.borrow_mut(), answer);
    }
    pub(crate) fn index(&self, count: usize, key: impl Fn(usize) -> u64) -> Index {
        postings::service::build(&mut self.postings.borrow_mut(), Count(count), |r| {
            CellWord(key(r.0))
        })
        .expect("file row count fits a posting ordinal")
    }
    pub(crate) fn index_for(&self, table: &Table, columns: &[usize]) -> Index {
        postings::service::build_columns(
            &mut self.postings.borrow_mut(),
            table.rows,
            &table.cols,
            postings::types::PostingColumns(columns),
            |&word| CellWord(word),
        )
        .expect("bound key columns cover the file rows")
    }
    pub(crate) fn index_rc(&self, index: Index) -> Rc<Index> {
        postings::service::share(&mut self.postings.borrow_mut(), index)
    }
    pub(crate) fn return_index(&self, index: Index) {
        postings::service::retire(&mut self.postings.borrow_mut(), index);
    }
    pub(crate) fn recycle_index(&self, index: Rc<Index>) {
        postings::service::release(&mut self.postings.borrow_mut(), index);
    }
    pub(crate) fn rows_with(&self, fill: impl FnOnce(&mut Vec<u32>)) -> Rc<Vec<u32>> {
        let mut rows = materialized::service::take_selection(&mut self.selections.borrow_mut());
        fill(Rc::get_mut(&mut rows).expect("worker retains only unique selections"));
        rows
    }
    pub(crate) fn recycle_rows(&self, rows: Rc<Vec<u32>>) {
        materialized::service::retire_selection(&mut self.selections.borrow_mut(), rows);
    }
    pub(crate) fn no_rows(&self) -> Rc<Vec<u32>> {
        self.selections.borrow().empty.clone()
    }
}

pub(crate) trait TableAccess {
    fn col(&self, name: &str) -> Result<usize, String>;
    fn push_row(&mut self, cells: &[u64]);
    fn row(&self, row: usize, out: &mut Vec<u64>);
}
impl TableAccess for Table {
    fn col(&self, name: &str) -> Result<usize, String> {
        materialized::service::position(self, name)
            .map(|at| at.0 as usize)
            .ok_or_else(|| format!("no column {name} in {:?}", self.names))
    }
    fn push_row(&mut self, cells: &[u64]) {
        materialized::service::append(self, cells)
    }
    fn row(&self, row: usize, out: &mut Vec<u64>) {
        materialized::service::gather(self, RowIndex(row), out).expect("valid input row");
    }
}
pub(crate) trait PostingAccess {
    fn get(&self, key: &u64) -> Option<&[u32]>;
}
impl PostingAccess for Index {
    #[inline]
    fn get(&self, key: &u64) -> Option<&[u32]> {
        postings::service::probe(self, CellWord(*key)).map(|p| p.0)
    }
}
pub(crate) fn key_of(words: impl Iterator<Item = u64>) -> u64 {
    postings::service::key(words.map(CellWord)).0
}
