//! The test support of the `table_model` concept: the rows its reads have handed out on this thread, and the representations' shared builders.

use crate::domain::table_model::collections::representations::{
    service::{borrowed, kind, registered},
    types::{CellRead, PhysicalShape, RepresentationKind, RowRead},
};
use crate::domain::table_model::types::{Rows, StoredSelection};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{CellKind, FactColumn, Integer, Relation, Value as Cell},
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
use std::sync::Arc;

thread_local! {
    /// Actual full-row reads for same-binary kernel differential tests only.
    static READ_ROW_COUNTS: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
}

/// Records one full-row read of `cells` cells.
pub(crate) fn note_read_row(cells: usize) {
    READ_ROW_COUNTS.with(|counts| {
        let (calls, total) = counts.get();
        counts.set((calls + 1, total + cells as u64));
    });
}

pub(crate) fn read_row_counts() -> (u64, u64) {
    READ_ROW_COUNTS.with(std::cell::Cell::get)
}

pub(crate) fn take_read_row_counts() -> (u64, u64) {
    READ_ROW_COUNTS.with(|counts| counts.replace((0, 0)))
}

pub(crate) fn compact(table: &Table) -> ryo_values::domain::table_words::types::CompactTable {
    use ryo_relations::domain::relations::types::Words;
    use ryo_values::domain::stored_facts::types::KeptWords;
    use ryo_values::domain::table_words::{
        service,
        types::{DecodeMode, WordSlice},
    };
    let mut words = Words::default();
    service::encode_table(table, &[], &mut words);
    let ryo_values::domain::table_words::types::Admitted {
        table: compact,
        names,
    } = ryo_values::domain::compact_table::service::admit_compact(
        ryo_values::domain::table_words::types::CompactBacking(Arc::new(KeptWords(service::pack(
            WordSlice(&words.0),
        )))),
        &mut Vec::new(),
        DecodeMode::Complete,
    )
    .unwrap();
    assert!(names.is_empty());
    assert_eq!(
        ryo_values::domain::compact_table::service::compact_rectangular(&compact),
        ryo_values::domain::table_words::types::TableShape::Rectangular
    );
    compact
}

pub(crate) fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}
pub(crate) fn column(values: &[i64]) -> FactColumn {
    FactColumn(
        values
            .iter()
            .map(|value| to_bits(number(*value)).0)
            .collect(),
    )
}
pub(crate) fn selection(
    rows: Vec<u32>,
    columns: Vec<ColumnIndex>,
    kinds: Vec<CellKind>,
) -> Arc<StoredSelection> {
    Arc::new(StoredSelection {
        relation: Relation::Function,
        rows: Arc::new(rows),
        columns,
        kinds,
        identity: Arc::new(Default::default()),
    })
}
pub(crate) fn values(read: RowRead<'_>) -> &[Cell] {
    match read {
        RowRead::Borrowed(row) | RowRead::Decoded(row) => row,
        RowRead::MissingPhysical => panic!("declared row missing"),
    }
}
pub(crate) fn check_rows(input: &Rows<'_>, expected: &[Vec<Cell>]) {
    let raw = borrowed(input);
    let item = registered(kind(raw));
    let contract = item.prepare(raw).expect("metadata");
    assert_eq!(item.inspect(&contract).unwrap(), PhysicalShape::Complete);
    assert_eq!(contract.rows, RowCount(expected.len()));
    assert_eq!(kind(item.bulk(raw).unwrap()), contract.kind);
    for (at, wanted) in expected.iter().enumerate() {
        let mut decode = vec![number(99); contract.kinds.len()];
        assert_eq!(
            values(item.row(raw, RowIndex(at), &mut decode).unwrap()),
            wanted
        );
        let mut decode = Vec::new();
        assert_eq!(
            crate::domain::table_model::service::read_row(input, RowIndex(at), &mut decode)
                .unwrap(),
            wanted
        );
        for (at_column, wanted) in wanted.iter().enumerate() {
            let at_column = ColumnIndex(u32::try_from(at_column).unwrap());
            assert_eq!(
                item.cell(raw, RowIndex(at), at_column).unwrap(),
                CellRead::Cell(*wanted)
            );
            assert_eq!(
                crate::domain::table_model::service::cell_at(input, RowIndex(at), at_column),
                Some(*wanted)
            );
        }
    }
    // A zero-column Stored view has no physical row bound; callers enumerate
    // only the zero rows declared by its contract, preserving legacy behavior.
    if contract.rows != RowCount(0) || contract.kind != RepresentationKind::Stored {
        let mut decode = vec![number(0); contract.kinds.len()];
        assert!(matches!(
            item.row(raw, RowIndex(usize::MAX), &mut decode).unwrap(),
            RowRead::MissingPhysical
        ));
    }
    assert_eq!(
        item.cell(raw, RowIndex(0), ColumnIndex(u32::MAX)).unwrap(),
        CellRead::MissingPhysical
    );
}
