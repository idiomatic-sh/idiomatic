//! Literal replacement and regex preparation errors use the real owner and matching contract.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::selects;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, HostedNumber, Input as PlanInput, LiteralText, StepId},
        plans::types::Operation,
    },
};
#[derive(Clone, Copy, Debug)]
struct Scenario {
    text: &'static str,
    pattern: &'static str,
    replacement: &'static str,
    expected: &'static str,
}
type Input = Scenario;
type Value = bool;
type Failure = OperatingError;
fn subject(input: Input) -> Result<Value, Failure> {
    let plan = inputs::plan(vec![
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::Replace {
            from: PlanInput::Step(StepId(0)),
            text: ColumnIndex(0),
            pattern: LiteralText(input.pattern.into()),
            replacement: LiteralText(input.replacement.into()),
        },
    ]);
    let mut symbols = Symbols::default();
    let original = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ryo_language::domain::symbols::types::ByteSlice(&[]),
        ryo_relations::domain::terms::types::NameText(input.text),
    );
    let output = fixtures::requests::published(
        plan,
        vec![],
        vec![fixtures::requests::texts("texts", &[input.text])],
        &mut symbols,
    )?;
    assert_eq!(output.count.0, 1);
    assert_eq!(output.cells.len(), 2);
    assert_eq!(
        output.cells[0],
        ryo_relations::domain::relations::types::Value::Symbol(original)
    );
    assert_eq!(inputs::text(output.cells[0], &symbols), input.text);
    assert_eq!(inputs::text(output.cells[1], &symbols), input.expected);
    Ok(true)
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all_whitespace_matches"),
        word: Word::Succeeds,
        input: || Input {
            text: "alpha \t beta\n gamma",
            pattern: r"\s+",
            replacement: "",
            expected: "alphabetagamma",
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unicode_boundaries"),
        word: Word::Succeeds,
        input: || Input {
            text: "é\u{2003}雪",
            pattern: r"\s+",
            replacement: "-",
            expected: "é-雪",
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("replacement_is_literal"),
        word: Word::Succeeds,
        input: || Input {
            text: "abc",
            pattern: "(b)",
            replacement: r"$1\x",
            expected: r"a$1\xc",
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("empty_match_progress"),
        word: Word::Succeeds,
        input: || Input {
            text: "é",
            pattern: "",
            replacement: "-",
            expected: "-é-",
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unmatched_text_preserved"),
        word: Word::Succeeds,
        input: || Input {
            text: "alpha",
            pattern: "z",
            replacement: "x",
            expected: "alpha",
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("invalid_pattern_refused"),
        word: Word::Refuses,
        input: || Input {
            text: "alpha",
            pattern: "[",
            replacement: "",
            expected: "",
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, OperatingError::InvalidRegex { .. }))
        }),
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
