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
use std::borrow::Cow;
#[derive(Clone, Copy, Debug)]
struct Scenario {
    text: &'static str,
    pattern: &'static str,
    replacement: &'static str,
}
type Input = Scenario;
/// What the replacement published: its rows and cells, whether the first cell is the original symbol, and every
/// first two cells' texts.
#[derive(Debug, PartialEq, Eq)]
struct Replaced {
    rows: usize,
    cells: usize,
    kept_original: bool,
    /// The texts of the first two cells.
    texts: [Option<Cow<'static, str>>; 2],
}
type Value = Replaced;
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
    Ok(Replaced {
        rows: output.count.0,
        cells: output.cells.len(),
        kept_original: output.cells.first()
            == Some(&ryo_relations::domain::relations::types::Value::Symbol(
                original,
            )),
        texts: [0, 1].map(|at| {
            output
                .cells
                .get(at)
                .map(|cell| Cow::Owned(inputs::text(*cell, &symbols).to_string()))
        }),
    })
}
/// One row whose first cell is the original `text` and second its replacement `replaced`.
const fn replaced(text: &'static str, replaced: &'static str) -> Expect<Value, Failure> {
    Expect::Value(Replaced {
        rows: 1,
        cells: 2,
        kept_original: true,
        texts: [Some(Cow::Borrowed(text)), Some(Cow::Borrowed(replaced))],
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all_whitespace_matches"),
        word: Word::Succeeds,
        input: || Input {
            text: "alpha \t beta\n gamma",
            pattern: r"\s+",
            replacement: "",
        },
        expect: replaced("alpha \t beta\n gamma", "alphabetagamma"),
    },
    Case {
        name: CaseName("unicode_boundaries"),
        word: Word::Succeeds,
        input: || Input {
            text: "é\u{2003}雪",
            pattern: r"\s+",
            replacement: "-",
        },
        expect: replaced("é\u{2003}雪", "é-雪"),
    },
    Case {
        name: CaseName("replacement_is_literal"),
        word: Word::Succeeds,
        input: || Input {
            text: "abc",
            pattern: "(b)",
            replacement: r"$1\x",
        },
        expect: replaced("abc", r"a$1\xc"),
    },
    Case {
        name: CaseName("empty_match_progress"),
        word: Word::Succeeds,
        input: || Input {
            text: "é",
            pattern: "",
            replacement: "-",
        },
        expect: replaced("é", "-é-"),
    },
    Case {
        name: CaseName("unmatched_text_preserved"),
        word: Word::Succeeds,
        input: || Input {
            text: "alpha",
            pattern: "z",
            replacement: "x",
        },
        expect: replaced("alpha", "alpha"),
    },
    Case {
        name: CaseName("invalid_pattern_refused"),
        word: Word::Refuses,
        input: || Input {
            text: "alpha",
            pattern: "[",
            replacement: "",
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
