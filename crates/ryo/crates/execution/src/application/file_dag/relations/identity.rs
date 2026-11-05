//! Shallow immutable-population witnesses; no row scan, hashing or retained provider borrow.
use super::SourceDescriptor;
use crate::domain::operator::contract::views::BorrowedRows;
use crate::domain::table_model::collections::representations::service::kind;
use ryo_relations::domain::relations::types::Relation;

pub(super) fn describe(relation: Relation, input: BorrowedRows<'_>) -> SourceDescriptor {
    let (storage, projection, source_kinds) = match input {
        BorrowedRows::Stored(view) => (
            view.columns
                .iter()
                .map(|column| (column.0.as_ptr() as usize, column.0.len()))
                .collect(),
            vec![],
            view.kinds.to_vec(),
        ),
        BorrowedRows::Selected {
            source,
            positions,
            columns,
            ..
        } => {
            let mut addresses = vec![(positions.as_ptr() as usize, positions.len())];
            addresses.extend(
                source
                    .columns
                    .iter()
                    .map(|column| (column.0.as_ptr() as usize, column.0.len())),
            );
            (addresses, columns.to_vec(), source.kinds.to_vec())
        }
        BorrowedRows::Compact(table) => (
            crate::domain::table_words::service::compact_storage(table).to_vec(),
            vec![],
            vec![],
        ),
        BorrowedRows::Computed(table) => (
            vec![
                (table as *const _ as usize, table.count.0),
                (table.cells.as_ptr() as usize, table.cells.len()),
            ],
            vec![],
            vec![],
        ),
        BorrowedRows::Positioned(view) => {
            let tree = describe(relation, BorrowedRows::Tree(view.tree));
            let mut storage = tree.storage;
            storage.push((
                view.positions.slots.0.as_ptr() as usize,
                view.positions.slots.0.len(),
            ));
            storage.push((view.shape as *const _ as usize, view.positions.width.0));
            (storage, vec![], vec![])
        }
        BorrowedRows::Tree(tree) => {
            let e = tree.elements;
            (
                vec![
                    (e as *const _ as usize, tree.file.0 as usize),
                    (e.kind.as_ptr() as usize, e.kind.len()),
                    (e.start.as_ptr() as usize, e.start.len()),
                    (e.end.as_ptr() as usize, e.end.len()),
                    (e.parent.as_ptr() as usize, e.parent.len()),
                    (e.ordinal.as_ptr() as usize, e.ordinal.len()),
                    (e.text.as_ptr() as usize, e.text.len()),
                    (e.size.as_ptr() as usize, e.size.len()),
                ],
                vec![],
                vec![],
            )
        }
    };
    SourceDescriptor {
        relation,
        kind: kind(input),
        kinds: input.kinds().to_vec(),
        rows: input.count(),
        storage,
        projection,
        source_kinds,
    }
}
pub(super) fn matches(source: &SourceDescriptor, input: BorrowedRows<'_>) -> bool {
    if source.kind != kind(input) || source.kinds != input.kinds() || source.rows != input.count() {
        return false;
    }
    let equal_columns = |columns: &[ryo_relations::domain::relations::types::FactColumn],
                         stored: &[(usize, usize)]| {
        columns.len() == stored.len()
            && columns.iter().zip(stored).all(|(column, (address, len))| {
                column.0.as_ptr() as usize == *address && column.0.len() == *len
            })
    };
    match input {
        BorrowedRows::Stored(view) => {
            source.source_kinds == view.kinds && equal_columns(view.columns, &source.storage)
        }
        BorrowedRows::Selected {
            source: view,
            positions,
            columns,
            ..
        } => {
            source.source_kinds == view.kinds
                && source.projection == columns
                && source.storage.first() == Some(&(positions.as_ptr() as usize, positions.len()))
                && equal_columns(view.columns, &source.storage[1..])
        }
        BorrowedRows::Compact(table) => {
            source.storage == crate::domain::table_words::service::compact_storage(table)
        }
        BorrowedRows::Computed(table) => {
            source.storage
                == [
                    (table as *const _ as usize, table.count.0),
                    (table.cells.as_ptr() as usize, table.cells.len()),
                ]
        }
        BorrowedRows::Positioned(view) => {
            let actual = describe(source.relation, BorrowedRows::Positioned(view));
            source.storage == actual.storage
        }
        BorrowedRows::Tree(tree) => {
            let e = tree.elements;
            source.storage
                == [
                    (e as *const _ as usize, tree.file.0 as usize),
                    (e.kind.as_ptr() as usize, e.kind.len()),
                    (e.start.as_ptr() as usize, e.start.len()),
                    (e.end.as_ptr() as usize, e.end.len()),
                    (e.parent.as_ptr() as usize, e.parent.len()),
                    (e.ordinal.as_ptr() as usize, e.ordinal.len()),
                    (e.text.as_ptr() as usize, e.text.len()),
                    (e.size.as_ptr() as usize, e.size.len()),
                ]
        }
    }
}
