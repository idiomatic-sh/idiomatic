//! The `cycles` item's cases.

use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::cycles::item::CyclesItem;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as Source;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;

const FROM: Source = Source::Relation(Relation::Function);

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

/// A table of numbers, one row for each slice.
fn numbers(rows: &[&[i64]]) -> Table {
    let width = rows.first().map_or(1, |row| row.len());
    let mut table = Table {
        kinds: vec![CellKind::Number; width],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter()
                .map(|number| Cell::Number(Integer(*number)))
                .collect::<Vec<_>>(),
        );
    }
    table
}

fn compute(
    operation: &Operation,
    inputs: &[&Table],
    bindings: &[ParameterBinding],
) -> Result<Value, OperatingError> {
    let mut symbols = Symbols::default();
    let rows: Vec<Rows<'_>> = inputs.iter().map(|table| Rows::Computed(table)).collect();
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    CyclesItem
        .compute(operation, &rows, &mut context, &mut out)
        .map(|()| out.count.0)
}

fn finds_one_cycle() -> Result<Value, OperatingError> {
    let operation = Operation::Cycles {
        edges: FROM,
        source: column(0),
        destination: column(1),
    };
    compute(&operation, &[&numbers(&[&[1, 2], &[2, 1], &[2, 3]])], &[])
}

fn finds_the_nodes_of_every_cycle_and_of_a_self_loop() -> Result<Value, OperatingError> {
    let operation = Operation::Cycles {
        edges: FROM,
        source: column(0),
        destination: column(1),
    };
    // 1 and 2 reach each other; 3, 4 and 5 are a ring; 6 reaches itself; 7 is only reached.
    compute(
        &operation,
        &[&numbers(&[
            &[1, 2],
            &[2, 1],
            &[2, 7],
            &[3, 4],
            &[4, 5],
            &[5, 3],
            &[6, 6],
        ])],
        &[],
    )
}

fn finds_no_node_on_a_path_that_does_not_return() -> Result<Value, OperatingError> {
    let operation = Operation::Cycles {
        edges: FROM,
        source: column(0),
        destination: column(1),
    };
    compute(&operation, &[&numbers(&[&[1, 2], &[2, 3], &[1, 3]])], &[])
}

fn finds_cycles_without_an_input() -> Result<Value, OperatingError> {
    let operation = Operation::Cycles {
        edges: FROM,
        source: column(0),
        destination: column(1),
    };
    compute(&operation, &[], &[])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_row_for_each_node_that_is_on_a_cycle"),
        word: Word::Succeeds,
        input: || finds_one_cycle,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_ring_and_a_self_loop_are_cycles_and_a_node_only_reached_is_not"),
        word: Word::Succeeds,
        input: || finds_the_nodes_of_every_cycle_and_of_a_self_loop,
        expect: Expect::Value(6),
    },
    Case {
        name: CaseName("a_path_that_does_not_return_is_no_cycle"),
        word: Word::Succeeds,
        input: || finds_no_node_on_a_path_that_does_not_return,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || finds_cycles_without_an_input,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::MissingInput { .. }))
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::Cycles,
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;

use ryo_values::domain::query_bindings::types::ParameterBinding;
