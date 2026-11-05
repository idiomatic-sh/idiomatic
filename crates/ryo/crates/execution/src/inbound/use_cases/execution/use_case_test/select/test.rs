//! Selection predicates and projections run through a genuine hosted input and file owner.
use super::super::super::{fixtures, types::OperatingError};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{
        Applied, ColumnIndex, Comparison, Function, HostedNumber, Input as Source, LiteralText,
        Operand, ParameterId, ParameterName, QueryLiteral, StepId, Test,
    },
    plans::types::{Operation, ParameterType, PlanParameter},
    records::types::Cell,
};
#[derive(Clone, Copy)]
enum Scenario {
    Numeric,
    Kept,
    Names,
    Ends,
    Unbound,
}
type Input = Scenario;
type Value = usize;
type Failure = OperatingError;
fn column(n: u32) -> ColumnIndex {
    ColumnIndex(n)
}
fn name() -> Operand {
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
fn services(test: Test, left: Operand, right: Operand) -> Result<usize, Failure> {
    let hosted = HostRelation {
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
    };
    let plan = fixtures::plan(vec![
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::Select {
            from: Source::Step(StepId(0)),
            comparisons: vec![Comparison {
                test,
                left,
                right,
                origin: None,
            }],
            kept: vec![],
        },
    ]);
    Ok(
        fixtures::published(plan, vec![], vec![hosted], &mut Symbols::default())?
            .count
            .0,
    )
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Names => Ok(
            services(Test::Equal, Operand::Column(column(0)), name())? * 10
                + services(Test::NotEqual, name(), Operand::Column(column(0)))?,
        ),
        Scenario::Ends => {
            let suffix = Operand::Literal(QueryLiteral::Text(LiteralText("Service".into())));
            let pascal = Operand::Function(Applied {
                function: Function::Pascal,
                arguments: vec![Operand::Column(column(1))],
            });
            Ok(
                services(Test::EndsWith, Operand::Column(column(0)), suffix)? * 100
                    + services(Test::StartsWith, Operand::Column(column(0)), pascal)? * 10
                    + services(Test::EndsWith, name(), Operand::Column(column(0)))?,
            )
        }
        _ => {
            let kept = matches!(scenario, Scenario::Kept);
            let mut plan = fixtures::plan(vec![
                Operation::Hosted {
                    relation: HostedNumber(0),
                },
                Operation::Select {
                    from: Source::Step(StepId(0)),
                    comparisons: vec![Comparison {
                        test: if matches!(scenario, Scenario::Unbound) {
                            Test::Equal
                        } else {
                            Test::GreaterThan
                        },
                        left: Operand::Column(column(u32::from(kept))),
                        right: if matches!(scenario, Scenario::Unbound) {
                            Operand::Parameter(ParameterId(0))
                        } else {
                            Operand::Literal(QueryLiteral::Number(Integer(if kept {
                                10
                            } else {
                                1
                            })))
                        },
                        origin: None,
                    }],
                    kept: if kept { vec![column(0)] } else { vec![] },
                },
            ]);
            if matches!(scenario, Scenario::Unbound) {
                plan.parameters.push(PlanParameter {
                    name: ParameterName("threshold".into()),
                    kind: ParameterType::Number,
                });
            }
            let rows: &[&[i64]] = if kept {
                &[&[1, 10], &[2, 20], &[3, 30]]
            } else {
                &[&[1], &[2], &[3]]
            };
            let out = fixtures::published(
                plan,
                vec![],
                vec![fixtures::numbers("numbers", if kept { 2 } else { 1 }, rows)],
                &mut Symbols::default(),
            )?;
            Ok(if kept {
                out.count.0 * 10 + out.kinds.len() + out.cells.len() * 100
            } else {
                out.count.0
            })
        }
    }
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("rows_that_satisfy_the_comparison_remain"),
        word: Word::Succeeds,
        input: || Scenario::Numeric,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("filtering_and_keeping_columns_is_one_step"),
        word: Word::Succeeds,
        input: || Scenario::Kept,
        expect: Expect::Value(221),
    },
    Case {
        name: CaseName("a_column_is_compared_with_a_function_of_another_column_row_by_row"),
        word: Word::Succeeds,
        input: || Scenario::Names,
        expect: Expect::Value(21),
    },
    Case {
        name: CaseName("starts_with_and_ends_with_take_columns_literals_and_functions"),
        word: Word::Succeeds,
        input: || Scenario::Ends,
        expect: Expect::Value(232),
    },
    Case {
        name: CaseName("an_unbound_parameter_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Unbound,
        expect: Expect::Error(OperatingError::Unbound),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
