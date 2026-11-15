//! The `ordered_match` kernel's cases, through the plan and run use cases: first in, first out within a key, and the
//! ordering's own refusal crossing whole.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::fixtures::{defines_rows, over_defines, over_nodes};
use crate::domain::physical_plan::types::PlanNodeId;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources, emitted};
use ryo_relations::domain::relations::types::{Offset, Relation, Span};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, JoinColumn, Side, StepId,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

/// The orders of Defines' rows: the start of each row's span.
type Input = &'static [u32];
/// The pairs as text, or the fault.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordered_match_pairs_first_in_first_out"),
        word: Word::Succeeds,
        input: || &[3, 1, 2],
        expect: Expect::Value(Cow::Borrowed(
            "Ok([[Number(Integer(1)), Number(Integer(2)), Number(Integer(2)), Number(Integer(3))]])",
        )),
    },
    Case {
        name: CaseName("ordered_match_refuses_a_duplicate_order"),
        word: Word::Succeeds,
        input: || &[1, 1],
        expect: Expect::Value(Cow::Borrowed("Err(Derived(DuplicateOrder))")),
    },
];

/// Each Defines row of one name ordered by its span's start, matched with itself: each left row with the earliest later row.
fn subject(orders: Input) -> Result<Value, Failure> {
    let mut resources = TextResources::new(&["k"]);
    let rows: Vec<_> = orders
        .iter()
        .map(|start| {
            (
                resources.symbol("k"),
                Span {
                    start: Offset(*start),
                    end: Offset(*start),
                },
            )
        })
        .collect();
    let table = defines_rows(&rows);
    let inputs = TableInputs::new(&[(Relation::Defines, &table)]);
    let bounds = || {
        over_defines(Operation::SpanBounds {
            from: StepInput::Relation(Relation::Defines),
            span: ColumnIndex(6),
        })
    };
    let column = |side, at| JoinColumn {
        side,
        column: ColumnIndex(at),
    };
    let steps = [
        bounds(),
        bounds(),
        over_nodes(
            Operation::OrderedMatch {
                left: StepInput::Step(StepId(0)),
                right: StepInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(3)],
                right_on: vec![ColumnIndex(3)],
                left_order: ColumnIndex(7),
                right_order: ColumnIndex(7),
                output: vec![column(Side::Left, 7), column(Side::Right, 7)],
            },
            &[0, 1],
        ),
    ];
    let answered = emitted(
        &steps,
        &[PlanNodeId(2)],
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    );
    Ok(Cow::Owned(format!(
        "{:?}",
        answered.map(|tables| tables
            .into_iter()
            .map(|table| table.cells)
            .collect::<Vec<_>>())
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
