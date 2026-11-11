//! The support of the `storage` cases.
use super::super::cases::fixtures::table_of;
use super::super::*;
#[cfg(test)]
use fixtures::execution::*;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_relations::domain::relations::{service::to_bits, types::FactColumn};
use ryo_values::domain::{plans::types::Operation, rows::types::Table};

pub(super) fn fact_column(values: &[i64]) -> FactColumn {
    FactColumn(
        values
            .iter()
            .map(|n| to_bits(Cell::Number(Integer(*n))).0)
            .collect(),
    )
}

pub(super) fn assert_representation_parity(
    op: &Operation,
    left: &Table,
    right: Rows<'_>,
    computed: &Table,
) {
    let expected = made(
        op,
        [Rows::Computed(left), Rows::Computed(computed)],
        &mut Symbols::default(),
        b"",
    )
    .unwrap();
    let actual = made(
        op,
        [Rows::Computed(left), right.clone()],
        &mut Symbols::default(),
        b"",
    )
    .unwrap();
    assert_eq!(actual, expected);
    let delivered = invoke(
        op,
        [Rows::Computed(left), right],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert_eq!(delivered.result.as_ref().unwrap().cells, expected.cells);
    assert_eq!(
        delivered
            .prefix
            .iter()
            .map(Vec::as_slice)
            .collect::<Vec<_>>(),
        expected
            .cells
            .chunks_exact(expected.kinds.len())
            .collect::<Vec<_>>()
    );
    let empty = table_of(&left.kinds, &[]);
    let skipped = invoke(
        op,
        [Rows::Computed(&empty), Rows::Computed(computed)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert!(skipped.result.unwrap().cells.is_empty());
    assert_eq!(skipped.work.index_rows, 0);
}
