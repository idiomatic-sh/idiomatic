//! A published table keeps its rows until the last reader releases it.
use super::{service, types::TableBuffers};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{CellKind, Count};
use ryo_values::domain::{descriptions::types::ColumnName, rows::types::RowIndex};
use std::rc::Rc;

type Input = usize;
type Value = usize;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("zero_slots_preserve_zero_width_rows"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("copies_and_posted_rows_survive_reuse"),
        word: Word::Succeeds,
        input: || 1,
        expect: Expect::Value(2),
    },
];

fn subject(width: Input) -> Result<Value, Failure> {
    let mut selections = super::types::SelectionBuffers::default();
    let mut selected = service::take_selection(&mut selections);
    Rc::get_mut(&mut selected).ok_or(())?.push(4_u32);
    let held = selected.clone();
    service::retire_selection(&mut selections, selected);
    assert_eq!(&held[..], &[4]);
    service::retire_selection(&mut selections, held);
    let fresh = service::take_selection(&mut selections);
    assert!(fresh.is_empty());
    assert!(fresh.capacity() > 0);
    let mut buffers: TableBuffers = TableBuffers::default();
    let names = Rc::new(vec![ColumnName("key".into()); width]);
    let kinds = Rc::new(vec![CellKind::Number; width]);
    let mut table = service::prepare(&mut buffers, names.clone(), kinds.clone(), Count(2));
    assert_eq!(table.rows, Count(0));
    for word in [CellWord(65), super::super::scalars::constants::MISSING_CELL] {
        service::append(&mut table, &[word][..width]);
    }
    let key = ColumnName("key".into());
    assert_eq!(
        service::position(&table, &key).map(|index| index.0),
        if width == 0 { None } else { Some(0) }
    );
    assert!(service::position(&table, &ColumnName("missing".into())).is_none());
    let mut duplicate = service::copy(&mut buffers, &table);
    if let Some(column) = duplicate.cols.first_mut() {
        column.reverse();
        assert_ne!(table.cols, duplicate.cols);
    }
    service::extend(&mut buffers, &mut duplicate, CellWord(17), Count(2));
    assert_eq!(
        duplicate.cols.last().ok_or(())?.as_slice(),
        &[CellWord(17); 2]
    );
    assert_eq!(table.cols.len(), width);
    service::release(&mut buffers, &mut duplicate);
    let first = service::share(&mut buffers, table);
    let reader = first.clone();
    assert!(Rc::ptr_eq(&reader, &first));
    service::retire(&mut buffers, first);
    let mut row = Vec::new();
    for (idx, word) in [(1, u64::MAX), (0, 65), (1, u64::MAX)] {
        service::gather(&reader, RowIndex(idx), &mut row).ok_or(())?;
        assert_eq!(row, [CellWord(word)][..width]);
    }
    assert!(service::gather(&reader, RowIndex(2), &mut row).is_none());
    let gathered = service::gather_rows(
        &mut buffers,
        &reader,
        [RowIndex(1), RowIndex(0), RowIndex(1)].into_iter(),
    )
    .ok_or(())?;
    assert_eq!(gathered.rows, Count(3));
    if let Some(column) = gathered.cols.first() {
        assert_eq!(
            &column[..],
            &[CellWord(u64::MAX), CellWord(65), CellWord(u64::MAX)]
        );
    }
    let count = reader.rows.0;
    let first_buffer = reader.cols.first().map(Vec::as_ptr);
    service::retire(&mut buffers, reader);
    let second = service::prepare(&mut buffers, names, kinds, Count(1));
    assert_eq!(second.rows, Count(0));
    assert_eq!(second.cols.first().map(Vec::as_ptr), first_buffer);
    assert!(second.cols.iter().all(Vec::is_empty));
    Ok(count)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
