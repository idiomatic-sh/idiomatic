//! Extend resolves values through the real request compiler and file owner.
use super::super::{fixtures, types::OperatingError};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            HostedNumber, Input as Source, LiteralText, Operand, ParameterId, ParameterName,
            QueryLiteral, StepId, WholeNumber,
        },
        plans::types::{Operation, ParameterType, PlanParameter},
        query_bindings::types::ParameterBinding,
    },
};
#[derive(Clone, Copy)]
enum Scenario {
    Number,
    Text,
    Unbound,
}
type Input = Scenario;
type Value = usize;
type Failure = OperatingError;
fn subject(scenario: Input) -> Result<Value, Failure> {
    let text = matches!(scenario, Scenario::Text);
    let values = if text {
        vec![
            Operand::Literal(QueryLiteral::Text(LiteralText("here".into()))),
            Operand::Parameter(ParameterId(0)),
        ]
    } else {
        vec![
            Operand::Literal(QueryLiteral::Number(Integer(1))),
            Operand::Parameter(ParameterId(0)),
        ]
    };
    let mut plan = fixtures::plan(vec![
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::Extend {
            from: Source::Step(StepId(0)),
            values,
        },
    ]);
    plan.parameters.push(PlanParameter {
        name: ParameterName("appended".into()),
        kind: if text {
            ParameterType::Text
        } else {
            ParameterType::Number
        },
    });
    let bindings = match scenario {
        Scenario::Number => vec![ParameterBinding::Number(WholeNumber(7))],
        Scenario::Text => vec![ParameterBinding::Text(LiteralText("here".into()))],
        Scenario::Unbound => vec![],
    };
    let mut symbols = Symbols::default();
    let rows: &[&[i64]] = if text { &[&[1], &[2]] } else { &[&[1]] };
    let out = fixtures::published(
        plan,
        bindings,
        vec![fixtures::numbers("input", 1, rows)],
        &mut symbols,
    )?;
    if text {
        assert_eq!(out.count.0, 2);
        assert_eq!(
            out.kinds,
            [CellKind::Number, CellKind::Symbol, CellKind::Symbol]
        );
        assert_eq!(fixtures::text(out.cells[1], &symbols), "here");
        assert_eq!(out.cells[1], out.cells[2]);
        assert_eq!(out.cells[2], out.cells[4]);
        assert_eq!(fixtures::text(out.cells[5], &symbols), "here");
        Ok(1)
    } else {
        assert_eq!(
            out.cells[1..],
            [
                ryo_relations::domain::relations::types::Value::Number(Integer(1)),
                ryo_relations::domain::relations::types::Value::Number(Integer(7))
            ]
        );
        Ok(out.count.0)
    }
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("appends_columns"),
        word: Word::Succeeds,
        input: || Scenario::Number,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_text_is_appended_as_a_name"),
        word: Word::Succeeds,
        input: || Scenario::Text,
        expect: Expect::Value(1),
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
