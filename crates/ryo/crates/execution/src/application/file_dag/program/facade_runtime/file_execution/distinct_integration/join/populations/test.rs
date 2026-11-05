//! Typed populations, dense bag order, partial provenance and fused reach semantics.
use super::super::cases::{node, stretch, table_of};
use super::super::*;
use ryo_values::domain::rows::service::{push_row, row_in};
fn functions(count: usize, dense: bool) -> Table {
    let mut table = Table {
        kinds: kinds(Relation::Function).to_vec(),
        ..Table::default()
    };
    for place in 0..count {
        push_row(
            &mut table,
            &[
                node(if dense { 1 } else { (place % 8 + 1) as u32 }),
                Cell::Symbol(Symbol(1)),
                Cell::File(FileId((place % 2) as u32)),
                stretch(place as u32, place as u32 + 1),
            ],
        );
    }
    table
}
#[test]
fn composite_typed_keys_keep_duplicate_bag_order_under_stored_and_zero_budget_routes() {
    let left = functions(256, false);
    let columns = (0..left.kinds.len())
        .map(|column| {
            FactColumn(
                left.cells
                    .chunks_exact(4)
                    .map(|row| to_bits(row[column]).0)
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let right = table_of(
        &[CellKind::Node, CellKind::File, CellKind::Number],
        &[
            &[node(3), Cell::File(FileId(0)), Cell::Number(Integer(30))],
            &[node(1), Cell::File(FileId(0)), Cell::Number(Integer(10))],
            &[node(3), Cell::File(FileId(0)), Cell::Number(Integer(31))],
            &[node(7), Cell::File(FileId(1)), Cell::Number(Integer(70))],
        ],
    );
    let op = operation(
        &[2, 0],
        &[1, 0],
        &[(Side::Left, 3), (Side::Right, 2), (Side::Left, 0)],
        vec![],
        &[],
    );
    let mut expected = vec![];
    for place in 0..256 {
        let at = place % 8 + 1;
        if at == 1 {
            expected.extend([
                stretch(place, place + 1),
                Cell::Number(Integer(10)),
                node(1),
            ]);
        }
        if at == 3 {
            for payload in [30, 31] {
                expected.extend([
                    stretch(place, place + 1),
                    Cell::Number(Integer(payload)),
                    node(3),
                ]);
            }
        }
    }
    for left in [
        Rows::Computed(&left),
        Rows::Stored(ColumnsView {
            kinds: kinds(Relation::Function),
            columns: &columns,
        }),
    ] {
        for optional_limit in [None, Some(0)] {
            let actual = invoke(
                &op,
                [left.clone(), Rows::Computed(&right)],
                &mut Symbols::default(),
                b"",
                false,
                real_port(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )
            .unwrap();
            assert_eq!(actual.result.unwrap().cells, expected);
            assert!(actual.affected.is_empty());
        }
    }
}
#[test]
fn dense_pairs_keep_all_65552_rows_and_first_last_order_without_optional_retention() {
    let left = functions(4097, true);
    let right = table_of(
        &[CellKind::Node, CellKind::Number],
        &(0..16)
            .map(|at| vec![node(1), Cell::Number(Integer(at))])
            .collect::<Vec<_>>()
            .iter()
            .map(Vec::as_slice)
            .collect::<Vec<_>>(),
    );
    let op = operation(
        &[0],
        &[0],
        &[(Side::Left, 3), (Side::Right, 1)],
        vec![],
        &[],
    );
    let actual = invoke(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        false,
        real_port(),
        Faults {
            optional_limit: Some(0),
            ..Faults::default()
        },
    )
    .unwrap();
    let table = actual.result.unwrap();
    assert_eq!(table.count, RowCount(4097 * 16));
    assert_eq!(
        row_in(&table, ryo_values::domain::rows::types::RowIndex(0)),
        Some(&[stretch(0, 1), Cell::Number(Integer(0))][..])
    );
    assert_eq!(
        row_in(
            &table,
            ryo_values::domain::rows::types::RowIndex(table.count.0 - 1)
        ),
        Some(&[stretch(4096, 4097), Cell::Number(Integer(15))][..])
    );
    for (place, row) in table.cells.chunks_exact(2).enumerate() {
        assert_eq!(
            row,
            [
                stretch((place / 16) as u32, (place / 16 + 1) as u32),
                Cell::Number(Integer((place % 16) as i64))
            ]
        );
    }
}
#[test]
fn partial_empty_population_keeps_exact_empty_rows_and_partial_provenance() {
    let left = functions(256, false);
    let right = table_of(&[CellKind::Node], &[&[node(99)]]);
    let actual = invoke(
        &operation(&[0], &[0], &[(Side::Left, 3)], vec![], &[]),
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        false,
        real_port(),
        Faults {
            partials: vec![crate::domain::coverage::types::Partial {
                relation: Relation::Entry,
                cause: ryo_values::domain::stored_facts::types::PartialCause(
                    "incomplete fixture".into(),
                ),
                lacking: crate::domain::coverage::types::Lacking::Any,
                at: None,
            }],
            optional_limit: Some(0),
            ..Faults::default()
        },
    )
    .unwrap();
    assert!(actual.result.unwrap().cells.is_empty());
    assert_eq!(actual.affected, vec![Some(FileId(0))]);
}
#[test]
fn reach_join_keeps_lazy_acquisition_exact_cycle_order_duplicates_and_refusals() {
    let n = |n| Cell::Number(Integer(n));
    let left = table_of(&[CellKind::Number], &[&[n(1)], &[n(1)]]);
    let empty = table_of(&[CellKind::Number], &[]);
    let right = table_of(
        &[CellKind::Number; 2],
        &[&[n(1), n(9)], &[n(1), n(2)], &[n(9), n(1)], &[n(2), n(3)]],
    );
    let mut op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let Operation::Join { reach, .. } = &mut op else {
        unreachable!()
    };
    *reach = Some(ReachLookup {
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        keys: vec![],
        columns: vec![ColumnIndex(0), ColumnIndex(1)],
    });
    let nothing = invoke(
        &op,
        [Rows::Computed(&empty), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert!(nothing.result.unwrap().cells.is_empty());
    assert_eq!(nothing.counts.0, 0);
    for (left_fault, right_fault, error) in [
        (None, Some(OperatingError::NotRead), OperatingError::NotRead),
        (Some(OperatingError::NotRun), None, OperatingError::NotRun),
    ] {
        let result = invoke(
            &op,
            [Rows::Computed(&left), Rows::Computed(&right)],
            &mut Symbols::default(),
            b"",
            true,
            real_port(),
            Faults {
                left: left_fault,
                right: right_fault,
                ..Faults::default()
            },
        )
        .unwrap();
        assert_eq!(
            format!("{:?}", result.result.unwrap_err()),
            format!("{error:?}")
        );
        assert!(result.prefix.is_empty());
    }
    let failed = invoke(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults {
            sink_after: Some(0),
            ..Faults::default()
        },
    )
    .unwrap();
    assert!(matches!(failed.result, Err(OperatingError::NotRun)));
    let actual = invoke(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert_eq!(
        actual.prefix,
        [1, 2, 3, 9, 1, 2, 3, 9].map(|at| vec![n(at)])
    );
    assert_eq!(actual.work.traversal_queue_len, 0);
}
