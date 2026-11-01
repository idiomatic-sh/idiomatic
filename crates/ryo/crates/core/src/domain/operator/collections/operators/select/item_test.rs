//! The `select` item's cases.

use ryo_language::domain::relations::types::{CellKind, Offset, Relation, Value as Cell};
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_values::domain::plans::types::QueryOperator;

use ryo_language::domain::symbols::types::Symbols;

use super::super::interface::OperatorsItem as _;
use super::item::SelectItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{ParameterBinding, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::Applied;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Function;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::ParameterId;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;

/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;
/// How many rows came out, or a number that says what the scenario observed.
type Value = usize;
type Failure = OperatingError;

const FROM: PlanInput = PlanInput::Relation(Relation::Function);

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

/// Computes `operation` over the table with nothing else in the context, into a fresh table.
fn compute(
    operation: &Operation,
    table: &Table,
    bindings: &[ParameterBinding],
) -> Result<Table, OperatingError> {
    let mut symbols = Symbols::default();
    let mut out = Table::default();
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings,
        symbols: &mut symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    SelectItem
        .compute(operation, &[Rows::Computed(table)], &mut context, &mut out)
        .map(|()| out)
}

/// A select over `FROM` of one comparison of column `left` with a number, keeping `kept`.
fn select(test: Test, left: u32, right: Operand, kept: Vec<ColumnIndex>) -> Operation {
    Operation::Select {
        from: FROM,
        comparisons: vec![Comparison {
            test,
            left: Operand::Column(column(left)),
            right,
            origin: None,
        }],
        kept,
    }
}

fn keeps_rows_that_satisfy_every_comparison() -> Result<Value, OperatingError> {
    let operation = select(
        Test::GreaterThan,
        0,
        Operand::Literal(QueryLiteral::Number(Integer(1))),
        Vec::new(),
    );
    compute(&operation, &numbers(&[&[1], &[2], &[3]]), &[]).map(|out| out.count.0)
}

/// Rows are filtered and cut to the columns kept in one step: the row count says the filter held,
/// the width says the columns were cut.
fn filters_and_keeps_columns() -> Result<Value, OperatingError> {
    let operation = select(
        Test::GreaterThan,
        1,
        Operand::Literal(QueryLiteral::Number(Integer(10))),
        vec![column(0)],
    );
    let out = compute(&operation, &numbers(&[&[1, 10], &[2, 20], &[3, 30]]), &[])?;
    Ok(out.count.0 * 10 + out.kinds.len() + out.cells.len() * 100)
}

fn compares_with_an_unbound_parameter() -> Result<Value, OperatingError> {
    let operation = select(
        Test::Equal,
        0,
        Operand::Parameter(ParameterId(0)),
        Vec::new(),
    );
    compute(&operation, &numbers(&[&[1]]), &[]).map(|out| out.count.0)
}

/// `concat(pascal(column 1), "Service")`: the name a service's class is given for its package.
fn service_name() -> Operand {
    Operand::Function(Applied {
        function: Function::Concat,
        arguments: vec![
            Operand::Function(Applied {
                function: Function::Pascal,
                arguments: vec![Operand::Column(column(1))],
            }),
            Operand::Literal(QueryLiteral::Text(LiteralText("Service".into()))),
        ],
    })
}

/// How many of the services' rows, each a class and its package, a select of `left test right` keeps.
fn services_held(test: Test, left: Operand, right: Operand) -> Result<Value, OperatingError> {
    let mut symbols = Symbols::default();
    let cells: Vec<[Cell; 2]> = SERVICE_ROWS
        .iter()
        .map(|pair| {
            pair.map(|(start, end)| {
                let at = (Offset(start), Offset(end));
                Cell::Symbol(intern(&mut symbols, ByteSlice(SERVICES), at.0, at.1))
            })
        })
        .collect();
    let mut table = Table {
        kinds: vec![CellKind::Symbol, CellKind::Symbol],
        ..Table::default()
    };
    for row in &cells {
        push_row(&mut table, row);
    }
    let mut out = Table::default();
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &[],
        symbols: &mut symbols,
        source: SourceView(SERVICES),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    let operation = Operation::Select {
        from: FROM,
        comparisons: vec![Comparison {
            test,
            left,
            right,
            origin: None,
        }],
        kept: Vec::new(),
    };
    SelectItem.compute(
        &operation,
        &[Rows::Computed(&table)],
        &mut context,
        &mut out,
    )?;
    Ok(out.count.0)
}

/// The class and the package of three services: `TodosService` in `todos`, `TodoListsService` in `todo_lists`, and
/// `BillingManager` in `billing`.
const SERVICES: &[u8] = b"TodosServicetodosTodoListsServicetodo_listsBillingManagerbilling";

/// The stretches of `SERVICES` that are each service's class and package.
const SERVICE_ROWS: [[(u32, u32); 2]; 3] = [
    [(0, 12), (12, 17)],
    [(17, 33), (33, 43)],
    [(43, 57), (57, 64)],
];

/// A class is named for its package as a function of the package's column says, row by row.
fn compares_a_column_with_a_function_of_another() -> Result<Value, OperatingError> {
    let named = services_held(Test::Equal, Operand::Column(column(0)), service_name())?;
    let misnamed = services_held(Test::NotEqual, service_name(), Operand::Column(column(0)))?;
    Ok(named * 10 + misnamed)
}

/// `starts_with` and `ends_with` take a column, a literal or a function on either side.
fn tells_prefixes_and_suffixes() -> Result<Value, OperatingError> {
    let suffix = Operand::Literal(QueryLiteral::Text(LiteralText("Service".into())));
    let ends = services_held(Test::EndsWith, Operand::Column(column(0)), suffix)?;
    let pascal = Operand::Function(Applied {
        function: Function::Pascal,
        arguments: vec![Operand::Column(column(1))],
    });
    let starts = services_held(Test::StartsWith, Operand::Column(column(0)), pascal)?;
    let column_suffix = services_held(Test::EndsWith, service_name(), Operand::Column(column(0)))?;
    Ok(ends * 100 + starts * 10 + column_suffix)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_that_satisfy_the_comparison_remain"),
        word: Word::Succeeds,
        input: || keeps_rows_that_satisfy_every_comparison,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("filtering_and_keeping_columns_is_one_step"),
        word: Word::Succeeds,
        input: || filters_and_keeps_columns,
        expect: Expect::Value(221),
    },
    Case {
        name: CaseName("a_column_is_compared_with_a_function_of_another_column_row_by_row"),
        word: Word::Succeeds,
        input: || compares_a_column_with_a_function_of_another,
        expect: Expect::Value(21),
    },
    Case {
        name: CaseName("starts_with_and_ends_with_take_columns_literals_and_functions"),
        word: Word::Succeeds,
        input: || tells_prefixes_and_suffixes,
        expect: Expect::Value(232),
    },
    Case {
        name: CaseName("an_unbound_parameter_is_refused"),
        word: Word::Refuses,
        input: || compares_with_an_unbound_parameter,
        expect: Expect::Error(OperatingError::Unbound),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    super::super::fixtures::contract::check(QueryOperator::Select);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
