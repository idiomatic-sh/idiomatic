//! Extend resolves values through the real request compiler and file owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            HostedNumber, Input as Source, LiteralText, Operand, ParameterId, ParameterName,
            QueryLiteral, StepId, WholeNumber,
        },
        plans::types::Operation,
        query_bindings::types::{ParameterBinding, ParameterType, PlanParameter},
    },
};
use std::borrow::Cow;
/// What the extend appends after each hosted number: a literal and a parameter of one type, the
/// parameter bound or not.
#[derive(Clone, Copy, Debug)]
struct Appending {
    text: bool,
    bound: bool,
    rows: &'static [&'static [i64]],
}

/// One published cell: a number, or the text of a name.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Shown {
    Number(i64),
    Text(Cow<'static, str>),
}

/// What the extend publishes: its column kinds, its rows, and whether every appended name is one name.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Extended {
    kinds: Cow<'static, [CellKind]>,
    rows: Cow<'static, [[Shown; 3]]>,
    one_name: bool,
}

type Input = Appending;
type Value = Extended;
type Failure = OperatingError;

/// The row of `number` followed by the text `here` twice.
const fn here(number: i64) -> [Shown; 3] {
    [
        Shown::Number(number),
        Shown::Text(Cow::Borrowed("here")),
        Shown::Text(Cow::Borrowed("here")),
    ]
}

/// Extends the hosted numbers and publishes them.
fn subject(appending: Input) -> Result<Value, Failure> {
    let text = appending.text;
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
    let mut plan = inputs::plan(vec![
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
    let bindings = match (appending.bound, text) {
        (false, _) => vec![],
        (true, false) => vec![ParameterBinding::Number(WholeNumber(7))],
        (true, true) => vec![ParameterBinding::Text(LiteralText("here".into()))],
    };
    let mut symbols = Symbols::default();
    let rows = appending.rows;
    let out = fixtures::requests::published(
        plan,
        bindings,
        vec![fixtures::requests::numbers("input", 1, rows)],
        &mut symbols,
    )?;
    let names: Vec<&Cell> = out
        .cells
        .iter()
        .filter(|cell| matches!(cell, Cell::Name(_) | Cell::Symbol(_)))
        .collect();
    let shown = |cell: &Cell| match cell {
        Cell::Number(Integer(number)) => Shown::Number(*number),
        other @ (Cell::Node(_)
        | Cell::Symbol(_)
        | Cell::Name(_)
        | Cell::File(_)
        | Cell::Span(_)
        | Cell::Flag(_)) => {
            Shown::Text(Cow::Owned(inputs::text(other.clone(), &symbols).to_owned()))
        }
    };
    Ok(Extended {
        kinds: Cow::Owned(out.kinds.clone()),
        rows: Cow::Owned(
            out.cells
                .chunks_exact(3)
                .map(|row| [shown(&row[0]), shown(&row[1]), shown(&row[2])])
                .collect(),
        ),
        one_name: names.iter().all(|name| *name == names[0]),
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("appends_columns"),
        word: Word::Succeeds,
        input: || Appending {
            text: false,
            bound: true,
            rows: &[&[1]],
        },
        expect: Expect::Value(Extended {
            kinds: Cow::Borrowed(&[CellKind::Number, CellKind::Number, CellKind::Number]),
            rows: Cow::Borrowed(&[[Shown::Number(1), Shown::Number(1), Shown::Number(7)]]),
            one_name: true,
        }),
    },
    Case {
        name: CaseName("a_text_is_appended_as_a_name"),
        word: Word::Succeeds,
        input: || Appending {
            text: true,
            bound: true,
            rows: &[&[1], &[2]],
        },
        expect: Expect::Value(Extended {
            kinds: Cow::Borrowed(&[CellKind::Number, CellKind::Symbol, CellKind::Symbol]),
            rows: Cow::Borrowed(&[here(1), here(2)]),
            one_name: true,
        }),
    },
    Case {
        name: CaseName("an_unbound_extend_parameter_is_refused"),
        word: Word::Refuses,
        input: || Appending {
            text: false,
            bound: false,
            rows: &[&[1]],
        },
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
