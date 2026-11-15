//! Buffers answer no rows for a node they do not hold, and keep answering none once it is released; `retain` keeps the rows its
//! predicate keeps, in their order, and every row when it keeps them all.
use super::value::FileBuffers;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::physical_plan::types::{NodeRows, PlanNodeId};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

#[derive(Debug, PartialEq)]
enum Held {
    Absent,
    /// The numbers of the rows held, in order.
    Materialized(Cow<'static, [i64]>),
}

type Input = Released;
type Value = Held;
type Failure = ();

/// What happens to node 0 before it is asked for.
#[derive(Clone, Copy)]
enum Released {
    /// It is never run.
    Never,
    /// It is released.
    Before,
    /// It holds 1, 2, 3, 2 and keeps every row.
    KeepsAll,
    /// It holds 1, 2, 3, 2 and keeps the rows that are not 2.
    DropsTwos,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_never_run_has_no_rows"),
        word: Word::Succeeds,
        input: || Released::Never,
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("a_released_node_has_no_rows"),
        word: Word::Succeeds,
        input: || Released::Before,
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("retaining_every_row_keeps_every_row_in_order"),
        word: Word::Succeeds,
        input: || Released::KeepsAll,
        expect: Expect::Value(Held::Materialized(Cow::Borrowed(&[1, 2, 3, 2]))),
    },
    Case {
        name: CaseName("retaining_drops_the_rows_it_does_not_keep_and_keeps_the_others_in_order"),
        word: Word::Succeeds,
        input: || Released::DropsTwos,
        expect: Expect::Value(Held::Materialized(Cow::Borrowed(&[1, 3]))),
    },
];

fn subject(released: Input) -> Result<Value, Failure> {
    let mut buffers = FileBuffers::default();
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for number in [1, 2, 3, 2] {
        push_row(&mut table, &[Cell::Number(Integer(number))]);
    }
    match released {
        Released::Never => {}
        Released::Before => buffers.release(PlanNodeId(0)),
        Released::KeepsAll => {
            buffers.nodes = vec![Some(table)];
            buffers.retain(PlanNodeId(0), |_| PredicateOutcome::Holds);
        }
        Released::DropsTwos => {
            buffers.nodes = vec![Some(table)];
            buffers.retain(PlanNodeId(0), |row| {
                if row == [Cell::Number(Integer(2))] {
                    PredicateOutcome::Fails
                } else {
                    PredicateOutcome::Holds
                }
            });
        }
    }
    Ok(match buffers.rows(PlanNodeId(0)) {
        NodeRows::Materialized(table) => Held::Materialized(Cow::Owned(
            table
                .cells
                .iter()
                .filter_map(|cell| {
                    if let Cell::Number(Integer(number)) = cell {
                        Some(*number)
                    } else {
                        None
                    }
                })
                .collect(),
        )),
        NodeRows::Absent => Held::Absent,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
