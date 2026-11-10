//! Stored/compact decoding and ordinary equality bag order through the compiled owner.
use super::super::cases::test::{node, stretch, table_of};
use super::super::*;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::service::row_in;
#[test]
fn exact_scalar_join_bag_order_reordered_and_empty_keys() {
    let n = |n| Cell::Number(Integer(n));
    let left = table_of(&[CellKind::Number; 2], &[&[n(1), n(10)], &[n(2), n(20)]]);
    let right = table_of(
        &[CellKind::Number; 2],
        &[&[n(1), n(100)], &[n(1), n(200)], &[n(3), n(300)]],
    );
    let op = operation(
        &[0],
        &[0],
        &[(Side::Left, 1), (Side::Right, 1)],
        vec![],
        &[],
    );
    let actual = made(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
    )
    .unwrap();
    assert_eq!(actual.count, RowCount(2));
    assert_eq!(actual.cells, [n(10), n(100), n(10), n(200)]);
}
fn fact_column(values: &[i64]) -> FactColumn {
    FactColumn(
        values
            .iter()
            .map(|n| to_bits(Cell::Number(Integer(*n))).0)
            .collect(),
    )
}

fn assert_representation_parity(op: &Operation, left: &Table, right: Rows<'_>, computed: &Table) {
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
#[test]
fn stored_right_composite_reordered_empty_and_unlike_keys_preserve_rows() {
    const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Flag, CellKind::Number];
    let columns = [
        fact_column(&[1, 2, 1, 3, 1]),
        FactColumn(vec![2, 0, 1, 1, 0]),
        fact_column(&[10, 20, 11, 30, 12]),
    ];
    let right = Rows::Stored(ColumnsView {
        kinds: KINDS,
        columns: &columns,
    });
    let mut computed = Table::default();
    materialize(&right, &mut computed, &mut vec![]);
    let n = |n| Cell::Number(Integer(n));
    let flag = |raised| Cell::Flag(if raised { Flag::Raised } else { Flag::Lowered });
    let left = table_of(
        &[CellKind::Number, CellKind::Flag],
        &[
            &[n(1), flag(true)],
            &[n(1), flag(false)],
            &[n(1), flag(true)],
            &[n(9), flag(true)],
        ],
    );
    for (l, r) in [
        (vec![0], vec![0]),
        (vec![1, 0], vec![1, 0]),
        (vec![0, 1], vec![0, 1]),
        (vec![], vec![]),
        (vec![0], vec![1]),
    ] {
        let op = operation(
            &l,
            &r,
            &[
                (Side::Right, 2),
                (Side::Left, 0),
                (Side::Right, 1),
                (Side::Right, 0),
                (Side::Right, 2),
            ],
            vec![],
            &[],
        );
        assert_representation_parity(&op, &left, right.clone(), &computed);
    }
}
#[test]
fn repeated_projected_positions_preserve_reordered_key_cells_and_flags() {
    const KINDS: &[CellKind] = &[CellKind::Number, CellKind::Flag, CellKind::Number];
    let columns = [
        fact_column(&[1, 2, 1, 3, 1]),
        FactColumn(vec![2, 0, 1, 1, 0]),
        fact_column(&[10, 20, 11, 30, 12]),
    ];
    let mut source = Table::default();
    materialize(
        &Rows::Stored(ColumnsView {
            kinds: KINDS,
            columns: &columns,
        }),
        &mut source,
        &mut vec![],
    );
    // These are expected fixture rows; the actual owner mints its selection from
    // the stored canonical provider and the authored Select, never a forged identity.
    let mut selected = table_of(&[CellKind::Number, CellKind::Number, CellKind::Flag], &[]);
    for at in [4, 1, 0, 4] {
        let row = row_in(&source, ryo_values::domain::rows::types::RowIndex(at)).unwrap();
        push_row(&mut selected, &[row[2], row[0], row[1]]);
    }
    let stored = (0..selected.kinds.len())
        .map(|column| {
            FactColumn(
                selected
                    .cells
                    .chunks_exact(selected.kinds.len())
                    .map(|row| to_bits(row[column]).0)
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let n = |n| Cell::Number(Integer(n));
    let left = table_of(
        &[CellKind::Number, CellKind::Flag],
        &[
            &[n(1), Cell::Flag(Flag::Lowered)],
            &[n(1), Cell::Flag(Flag::Raised)],
        ],
    );
    for (l, r) in [
        (vec![0, 1], vec![1, 2]),
        (vec![1, 0], vec![2, 1]),
        (vec![], vec![]),
    ] {
        let op = operation(
            &l,
            &r,
            &[(Side::Right, 0), (Side::Right, 0), (Side::Left, 0)],
            vec![],
            &[],
        );
        assert_representation_parity(
            &op,
            &left,
            Rows::Stored(ColumnsView {
                kinds: &[CellKind::Number, CellKind::Number, CellKind::Flag],
                columns: &stored,
            }),
            &selected,
        );
    }
}
#[test]
fn malformed_storage_materializes_its_complete_prefix_before_owner_admission() {
    let columns = [fact_column(&[1, 1, 1]), fact_column(&[10, 11])];
    let mut logical = Table::default();
    materialize(
        &Rows::Stored(ColumnsView {
            kinds: &[CellKind::Number; 2],
            columns: &columns,
        }),
        &mut logical,
        &mut vec![],
    );
    assert_eq!(logical.count, RowCount(2));
    assert_eq!(
        logical.cells,
        [1, 10, 1, 11].map(|n| Cell::Number(Integer(n)))
    );
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(1))]]);
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let out = made(
        &op,
        [Rows::Computed(&left), Rows::Computed(&logical)],
        &mut Symbols::default(),
        b"",
    )
    .unwrap();
    assert_eq!(out.cells, [10, 11].map(|n| Cell::Number(Integer(n))));
}
#[test]
fn unsorted_duplicate_rows_sink_prefix_and_source_refusals_are_exact() {
    let n = |n| Cell::Number(Integer(n));
    let left = table_of(&[CellKind::Number], &[&[n(2)]]);
    let columns = [fact_column(&[2, 1, 2, 2]), fact_column(&[20, 10, 21, 22])];
    let right = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns: &columns,
    });
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let good = invoke(
        &op,
        [Rows::Computed(&left), right.clone()],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert_eq!(good.prefix, vec![vec![n(20)], vec![n(21)], vec![n(22)]]);
    assert!(good.result.is_ok());
    assert_eq!(good.work.index_cells, 0);
    let sink = invoke(
        &op,
        [Rows::Computed(&left), right.clone()],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults {
            sink_after: Some(1),
            ..Faults::default()
        },
    )
    .unwrap();
    assert_eq!(sink.result, Err(OperatingError::NotRun));
    assert_eq!(sink.prefix, vec![vec![n(20)]]);
    for (fault, error) in [
        (
            Faults {
                left: Some(OperatingError::NotRun),
                ..Faults::default()
            },
            OperatingError::NotRun,
        ),
        (
            Faults {
                right: Some(OperatingError::NotRead),
                ..Faults::default()
            },
            OperatingError::NotRead,
        ),
    ] {
        let refused = invoke(
            &op,
            [Rows::Computed(&left), right.clone()],
            &mut Symbols::default(),
            b"",
            true,
            real_port(),
            fault,
        )
        .unwrap();
        assert_eq!(refused.result, Err(error));
        assert!(refused.prefix.is_empty());
        assert_eq!(refused.work.index_rows, 0);
    }
}

#[test]
fn malformed_provider_is_normalized_by_the_real_owner_before_joining_its_complete_prefix() {
    let n = |n| Cell::Number(Integer(n));
    let left = table_of(&[CellKind::Number], &[&[n(1)]]);
    let right = table_of(
        &[CellKind::Number; 2],
        &[&[n(1), n(10)], &[n(1), n(11)], &[n(1), n(12)]],
    );
    let mut steps = vec![];
    let (left, l) = provider(Rows::Computed(&left), 0, &mut steps);
    let right_columns = [FactColumn(vec![1, 1, 1]), FactColumn(vec![10, 11, 12])];
    let (mut right, r) = provider(
        Rows::Stored(ColumnsView {
            kinds: &[CellKind::Number; 2],
            columns: &right_columns,
        }),
        1,
        &mut steps,
    );
    let Operation::Select { kept, .. } = &steps[1].operation else {
        panic!("the fixture builds a Select")
    };
    right.columns[kept[1].0 as usize].0.truncate(2);
    let mut join = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let Operation::Join {
        left: from,
        right: other,
        ..
    } = &mut join
    else {
        panic!("the fixture builds a Join")
    };
    *from = l;
    *other = r;
    steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: join,
    });
    let mut request = plan(Scenario::ProjectRetry);
    request.steps = steps;
    request.emit.step = StepId(2);
    let actual = execute(
        request,
        [left, right],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert_eq!(actual.prefix, vec![vec![n(10)], vec![n(11)]]);
    assert!(actual.result.is_ok());
}
