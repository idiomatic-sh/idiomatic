//! Selection predicates and projections run through a genuine hosted input and file owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Integer, Value as Published};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{
        Applied, ColumnIndex, Function, HostedNumber, Input as Source, LiteralText, Operand,
        ParameterId, ParameterName, PlanComparison, QueryLiteral, StepId, Test,
    },
    plans::types::Operation,
    query_bindings::types::{ParameterType, PlanParameter},
    row_texts::types::Cell,
};
use std::borrow::Cow;
/// One side of a comparison over the services table: its class column, its package column in
/// Pascal case, the service name made from it, or the literal `Service`.
#[derive(Clone, Copy, Debug)]
enum Side {
    Class,
    PascalPackage,
    ServiceName,
    Suffix,
}

/// A threshold over hosted numbers: `left > right` where `right` is a literal, or `left = ?` where
/// the parameter is unbound.
#[derive(Clone, Copy, Debug)]
enum Threshold {
    Literal(i64),
    Unbound,
}

/// What a row selects.
#[derive(Clone, Copy, Debug)]
enum Selection {
    /// The hosted number rows filtered on column `left` by the threshold, keeping `kept` or every column.
    Numbers {
        rows: &'static [&'static [i64]],
        left: u32,
        threshold: Threshold,
        kept: Option<u32>,
    },
    /// The services table (`TodosService`/`todos`, `TodoListsService`/`todo_lists`,
    /// `BillingManager`/`billing`) filtered on one comparison.
    Services { test: Test, left: Side, right: Side },
}

/// One published cell: a number, or the text of a name.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Shown {
    Number(i64),
    Text(Cow<'static, str>),
}

type Input = Selection;
/// The rows the selection publishes.
type Value = Cow<'static, [Cow<'static, [Shown]>]>;
type Failure = OperatingError;

/// The `TodosService` row.
const TODOS: Cow<'static, [Shown]> = Cow::Borrowed(&[
    Shown::Text(Cow::Borrowed("TodosService")),
    Shown::Text(Cow::Borrowed("todos")),
]);
/// The `TodoListsService` row.
const TODO_LISTS: Cow<'static, [Shown]> = Cow::Borrowed(&[
    Shown::Text(Cow::Borrowed("TodoListsService")),
    Shown::Text(Cow::Borrowed("todo_lists")),
]);
/// The `BillingManager` row.
const BILLING: Cow<'static, [Shown]> = Cow::Borrowed(&[
    Shown::Text(Cow::Borrowed("BillingManager")),
    Shown::Text(Cow::Borrowed("billing")),
]);
/// The numbers 1, 2 and 3.
const ONE_TWO_THREE: &[&[i64]] = &[&[1], &[2], &[3]];

/// A published row of `values`.
const fn numbers(values: &'static [Shown]) -> Cow<'static, [Shown]> {
    Cow::Borrowed(values)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_that_satisfy_the_comparison_remain"),
        word: Word::Succeeds,
        input: || Selection::Numbers {
            rows: ONE_TWO_THREE,
            left: 0,
            threshold: Threshold::Literal(1),
            kept: None,
        },
        expect: Expect::Value(Cow::Borrowed(&[
            numbers(&[Shown::Number(2)]),
            numbers(&[Shown::Number(3)]),
        ])),
    },
    Case {
        name: CaseName("filtering_and_keeping_columns_is_one_step"),
        word: Word::Succeeds,
        input: || Selection::Numbers {
            rows: &[&[1, 10], &[2, 20], &[3, 30]],
            left: 1,
            threshold: Threshold::Literal(10),
            kept: Some(0),
        },
        expect: Expect::Value(Cow::Borrowed(&[
            numbers(&[Shown::Number(2)]),
            numbers(&[Shown::Number(3)]),
        ])),
    },
    Case {
        name: CaseName("a_column_is_compared_with_a_function_of_another_column_row_by_row"),
        word: Word::Succeeds,
        input: || Selection::Services {
            test: Test::Equal,
            left: Side::Class,
            right: Side::ServiceName,
        },
        expect: Expect::Value(Cow::Borrowed(&[TODOS, TODO_LISTS])),
    },
    Case {
        name: CaseName("a_function_of_a_column_is_unequal_to_another_column_row_by_row"),
        word: Word::Succeeds,
        input: || Selection::Services {
            test: Test::NotEqual,
            left: Side::ServiceName,
            right: Side::Class,
        },
        expect: Expect::Value(Cow::Borrowed(&[BILLING])),
    },
    Case {
        name: CaseName("starts_with_and_ends_with_take_columns_literals_and_functions"),
        word: Word::Succeeds,
        input: || Selection::Services {
            test: Test::EndsWith,
            left: Side::Class,
            right: Side::Suffix,
        },
        expect: Expect::Value(Cow::Borrowed(&[TODOS, TODO_LISTS])),
    },
    Case {
        name: CaseName("starts_with_takes_a_function_of_a_column"),
        word: Word::Succeeds,
        input: || Selection::Services {
            test: Test::StartsWith,
            left: Side::Class,
            right: Side::PascalPackage,
        },
        expect: Expect::Value(Cow::Borrowed(&[TODOS, TODO_LISTS, BILLING])),
    },
    Case {
        name: CaseName("ends_with_takes_a_function_on_its_left"),
        word: Word::Succeeds,
        input: || Selection::Services {
            test: Test::EndsWith,
            left: Side::ServiceName,
            right: Side::Class,
        },
        expect: Expect::Value(Cow::Borrowed(&[TODOS, TODO_LISTS])),
    },
    Case {
        name: CaseName("an_unbound_select_parameter_is_refused"),
        word: Word::Refuses,
        input: || Selection::Numbers {
            rows: ONE_TWO_THREE,
            left: 0,
            threshold: Threshold::Unbound,
            kept: None,
        },
        expect: Expect::Error(OperatingError::Unbound),
    },
];

/// The operand a side names.
fn operand(side: Side) -> Operand {
    let pascal = || {
        Operand::Function(Applied {
            function: Function::Pascal,
            arguments: vec![Operand::Column(ColumnIndex(1))],
        })
    };
    let suffix = || Operand::Literal(QueryLiteral::Text(LiteralText("Service".into())));
    match side {
        Side::Class => Operand::Column(ColumnIndex(0)),
        Side::PascalPackage => pascal(),
        Side::ServiceName => Operand::Function(Applied {
            function: Function::Concat,
            arguments: vec![pascal(), suffix()],
        }),
        Side::Suffix => suffix(),
    }
}

/// The services table.
fn services() -> HostRelation {
    HostRelation {
        name: HostName("services".into()),
        columns: ["class", "package"]
            .into_iter()
            .map(|n| HostColumn {
                name: HostColumnName(n.into()),
                kind: HostKind::Text,
            })
            .collect(),
        rows: [
            ["TodosService", "todos"],
            ["TodoListsService", "todo_lists"],
            ["BillingManager", "billing"],
        ]
        .into_iter()
        .map(|row| HostRow(row.into_iter().map(|s| Cell::Text(s.into())).collect()))
        .collect(),
    }
}

/// Selects the hosted rows on the comparison and publishes them.
fn subject(selection: Input) -> Result<Value, Failure> {
    let (comparison, kept, hosted, parameters) = match selection {
        Selection::Services { test, left, right } => (
            (test, operand(left), operand(right)),
            vec![],
            services(),
            vec![],
        ),
        Selection::Numbers {
            rows,
            left,
            threshold,
            kept,
        } => {
            let width = rows.first().map_or(0, |row| row.len());
            let hosted = fixtures::requests::numbers("numbers", width, rows);
            let left = Operand::Column(ColumnIndex(left));
            let (comparison, parameters) = match threshold {
                Threshold::Literal(value) => (
                    (
                        Test::GreaterThan,
                        left,
                        Operand::Literal(QueryLiteral::Number(Integer(value))),
                    ),
                    vec![],
                ),
                Threshold::Unbound => (
                    (Test::Equal, left, Operand::Parameter(ParameterId(0))),
                    vec![PlanParameter {
                        name: ParameterName("threshold".into()),
                        kind: ParameterType::Number,
                    }],
                ),
            };
            (
                comparison,
                kept.map(ColumnIndex).into_iter().collect(),
                hosted,
                parameters,
            )
        }
    };
    let (test, left, right) = comparison;
    let mut plan = inputs::plan(vec![
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::Select {
            from: Source::Step(StepId(0)),
            comparisons: vec![PlanComparison {
                test,
                left,
                right,
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            }],
            kept,
        },
    ]);
    plan.parameters = parameters;
    let mut symbols = Symbols::default();
    let out = fixtures::requests::published(plan, vec![], vec![hosted], &mut symbols)?;
    let shown = |cell: &Published| match cell {
        Published::Number(Integer(number)) => Shown::Number(*number),
        other => Shown::Text(Cow::Owned(inputs::text(other.clone(), &symbols).to_owned())),
    };
    let width = out.kinds.len().max(1);
    Ok(Cow::Owned(
        out.cells
            .chunks_exact(width)
            .map(|row| Cow::Owned(row.iter().map(shown).collect()))
            .collect(),
    ))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
