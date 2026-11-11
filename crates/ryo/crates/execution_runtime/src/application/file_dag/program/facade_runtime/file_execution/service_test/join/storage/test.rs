//! Stored/compact decoding and ordinary equality bag order through the compiled owner.
use super::super::cases::fixtures::table_of;
use super::super::*;
use super::fixtures::{
    Fault, Join, Observed, Parity, Refused, fact_column, malformed_provider, parity, unsorted,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use fixtures::execution::*;
use fixtures::provider::operation;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::service::materialize;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Flag, Integer, Value as Cell};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::service::row_in;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

type Input = Join;
type Value = Observed;
type Failure = Refused;

/// A number cell.
const fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("exact_scalar_join_bag_order_reordered_and_empty_keys"),
        word: Word::Succeeds,
        input: || Join::Scalar,
        expect: Expect::Value(Observed::Rows {
            rows: 2,
            cells: Cow::Borrowed(&[number(10), number(100), number(10), number(200)]),
        }),
    },
    Case {
        name: CaseName("stored_right_composite_reordered_empty_and_unlike_keys_preserve_rows"),
        word: Word::Succeeds,
        input: || Join::StoredRight(&[0], &[0]),
        expect: Expect::Value(Observed::Parity(Parity {
            rows: 9,
            made_equal: true,
            delivered_equal: (true, true),
            skipped: (0, 0),
        })),
    },
    Case {
        name: CaseName("a_stored_right_side_on_a_composite_key_preserves_rows"),
        word: Word::Succeeds,
        input: || Join::StoredRight(&[1, 0], &[1, 0]),
        expect: Expect::Value(Observed::Parity(Parity {
            rows: 5,
            made_equal: true,
            delivered_equal: (true, true),
            skipped: (0, 0),
        })),
    },
    Case {
        name: CaseName("a_stored_right_side_on_a_reordered_composite_key_preserves_rows"),
        word: Word::Succeeds,
        input: || Join::StoredRight(&[0, 1], &[0, 1]),
        expect: Expect::Value(Observed::Parity(Parity {
            rows: 5,
            made_equal: true,
            delivered_equal: (true, true),
            skipped: (0, 0),
        })),
    },
    Case {
        name: CaseName("a_stored_right_side_without_keys_preserves_rows"),
        word: Word::Succeeds,
        input: || Join::StoredRight(&[], &[]),
        expect: Expect::Value(Observed::Parity(Parity {
            rows: 20,
            made_equal: true,
            delivered_equal: (true, true),
            skipped: (0, 0),
        })),
    },
    Case {
        name: CaseName("a_stored_right_side_on_unlike_keys_preserves_rows"),
        word: Word::Succeeds,
        input: || Join::StoredRight(&[0], &[1]),
        expect: Expect::Value(Observed::Parity(Parity {
            rows: 0,
            made_equal: true,
            delivered_equal: (true, true),
            skipped: (0, 0),
        })),
    },
    Case {
        name: CaseName("repeated_projected_positions_preserve_reordered_key_cells_and_flags"),
        word: Word::Succeeds,
        input: || Join::Repeated(&[0, 1], &[1, 2]),
        expect: Expect::Value(Observed::Parity(Parity {
            rows: 3,
            made_equal: true,
            delivered_equal: (true, true),
            skipped: (0, 0),
        })),
    },
    Case {
        name: CaseName("repeated_projected_positions_on_a_reordered_key_preserve_cells_and_flags"),
        word: Word::Succeeds,
        input: || Join::Repeated(&[1, 0], &[2, 1]),
        expect: Expect::Value(Observed::Parity(Parity {
            rows: 3,
            made_equal: true,
            delivered_equal: (true, true),
            skipped: (0, 0),
        })),
    },
    Case {
        name: CaseName("repeated_projected_positions_without_keys_preserve_cells_and_flags"),
        word: Word::Succeeds,
        input: || Join::Repeated(&[], &[]),
        expect: Expect::Value(Observed::Parity(Parity {
            rows: 8,
            made_equal: true,
            delivered_equal: (true, true),
            skipped: (0, 0),
        })),
    },
    Case {
        name: CaseName("malformed_storage_materializes_its_complete_prefix_before_owner_admission"),
        word: Word::Succeeds,
        input: || Join::MalformedStorage,
        expect: Expect::Value(Observed::Materialized {
            rows: 2,
            logical: Cow::Borrowed(&[number(1), number(10), number(1), number(11)]),
            joined: Cow::Borrowed(&[number(10), number(11)]),
        }),
    },
    Case {
        name: CaseName("unsorted_duplicate_rows_sink_prefix_and_source_refusals_are_exact"),
        word: Word::Succeeds,
        input: || Join::Unsorted(Fault::None),
        expect: Expect::Value(Observed::Delivered {
            prefix: Cow::Borrowed(&[number(20), number(21), number(22)]),
            index_cells: 0,
        }),
    },
    Case {
        name: CaseName("an_unsorted_duplicate_join_keeps_the_sink_prefix_before_its_refusal"),
        word: Word::Refuses,
        input: || Join::Unsorted(Fault::SinkAfterOne),
        expect: Expect::Error(Refused {
            error: OperatingError::NotRun,
            prefix: Cow::Borrowed(&[number(20)]),
            index_rows: 4,
        }),
    },
    Case {
        name: CaseName("an_unsorted_duplicate_join_keeps_the_left_source_refusal"),
        word: Word::Refuses,
        input: || Join::Unsorted(Fault::Left),
        expect: Expect::Error(Refused {
            error: OperatingError::NotRun,
            prefix: Cow::Borrowed(&[]),
            index_rows: 0,
        }),
    },
    Case {
        name: CaseName("an_unsorted_duplicate_join_keeps_the_right_source_refusal"),
        word: Word::Refuses,
        input: || Join::Unsorted(Fault::Right),
        expect: Expect::Error(Refused {
            error: OperatingError::NotRead,
            prefix: Cow::Borrowed(&[]),
            index_rows: 0,
        }),
    },
    Case {
        name: CaseName(
            "malformed_provider_is_normalized_by_the_real_owner_before_joining_its_complete_prefix",
        ),
        word: Word::Succeeds,
        input: || Join::MalformedProvider,
        expect: Expect::Value(Observed::Delivered {
            prefix: Cow::Borrowed(&[number(10), number(11)]),
            index_cells: 0,
        }),
    },
];

/// Runs the Join.
fn exercise(join: Join) -> Result<Observed, Refused> {
    match join {
        Join::Scalar => Ok(scalar()),
        Join::StoredRight(l, r) => Ok(stored_right(l, r)),
        Join::Repeated(l, r) => Ok(repeated(l, r)),
        Join::MalformedStorage => Ok(malformed_storage()),
        Join::Unsorted(fault) => unsorted(fault),
        Join::MalformedProvider => malformed_provider(),
    }
}

/// Joins two scalar rows to three, keyed on the first column.
fn scalar() -> Observed {
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
    Observed::Rows {
        rows: actual.count.0,
        cells: Cow::Owned(actual.cells),
    }
}

/// Joins four left rows to a stored right side on the keys `l` and `r`.
fn stored_right(l: &[u32], r: &[u32]) -> Observed {
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
    {
        let op = operation(
            l,
            r,
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
        Observed::Parity(parity(&op, &left, right, &computed))
    }
}
/// Joins two left rows to a stored selection that repeats its projected positions, on `l` and `r`.
fn repeated(l: &[u32], r: &[u32]) -> Observed {
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
    {
        let op = operation(
            l,
            r,
            &[(Side::Right, 0), (Side::Right, 0), (Side::Left, 0)],
            vec![],
            &[],
        );
        Observed::Parity(parity(
            &op,
            &left,
            Rows::Stored(ColumnsView {
                kinds: &[CellKind::Number, CellKind::Number, CellKind::Flag],
                columns: &stored,
            }),
            &selected,
        ))
    }
}
/// Materializes stored columns of three and two rows, then joins the result.
fn malformed_storage() -> Observed {
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
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(1))]]);
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let out = made(
        &op,
        [Rows::Computed(&left), Rows::Computed(&logical)],
        &mut Symbols::default(),
        b"",
    )
    .unwrap();
    Observed::Materialized {
        rows: logical.count.0,
        logical: Cow::Owned(logical.cells),
        joined: Cow::Owned(out.cells),
    }
}
/// Runs the Join.
fn subject(join: Input) -> Result<Value, Failure> {
    exercise(join)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
