//! Text row operators retain the original row and publish exact appended words or slots.
use super::super::super::fixtures;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            CapturePattern, ColumnIndex, HostedNumber, Input as PlanInput, StepId,
        },
        plans::types::Operation,
    },
};
use std::borrow::Cow;
#[derive(Clone, Copy)]
enum Scenario {
    Words,
    NoWords,
    MissingWords,
    CaptureModule,
    CaptureStarred,
    CaptureSplits,
    MissingCapture,
}
type Input = Scenario;
type Value = (usize, usize, Cow<'static, [Cow<'static, str>]>);
type Failure = OperatingError;
fn subject(input: Input) -> Result<Value, Failure> {
    let missing = matches!(input, Scenario::MissingWords | Scenario::MissingCapture);
    let words = matches!(
        input,
        Scenario::Words | Scenario::NoWords | Scenario::MissingWords
    );
    let originals: &[&str] = match input {
        Scenario::Words => &["Hello wor_ld, PythonProvider v2Beta"],
        Scenario::NoWords => &[" -- ... "],
        Scenario::CaptureModule => &["src/lib.rs", "src/deep/mod.rs", "README.md"],
        Scenario::CaptureStarred => &["a/b/languages/py/src/x.rs", "languages/py/x.r", ""],
        Scenario::CaptureSplits => &["x/y/z.rs"],
        _ => &[],
    };
    let mut operations = vec![];
    let hosted = if missing {
        vec![]
    } else {
        operations.push(Operation::Hosted {
            relation: HostedNumber(0),
        });
        vec![fixtures::requests::texts("texts", originals)]
    };
    let from = if missing {
        PlanInput::Relation(Relation::Function)
    } else {
        PlanInput::Step(StepId(0))
    };
    let text = ColumnIndex(if missing { 1 } else { 0 });
    let pattern = match input {
        Scenario::CaptureModule => "src/{module}.rs",
        Scenario::CaptureStarred => "{*above}/languages/{name}/{*below}",
        _ => "{*ancestor}/{*rest}",
    };
    operations.push(if words {
        Operation::Words { from, text }
    } else {
        Operation::Capture {
            from,
            path: text,
            pattern: CapturePattern(pattern.into()),
        }
    });
    let mut symbols = Symbols::default();
    let output = fixtures::requests::published(inputs::plan(operations), vec![], hosted, &mut symbols)?;
    let width = output.kinds.len();
    let mut appended = Vec::new();
    for row in output.cells.chunks(width) {
        let original = inputs::text(row[0], &symbols);
        assert!(
            originals.contains(&original),
            "every output retains an original input text"
        );
        appended.extend(
            row[1..]
                .iter()
                .map(|cell| Cow::Owned(inputs::text(*cell, &symbols).to_owned())),
        );
    }
    assert_eq!(output.cells.len(), output.count.0 * width);
    Ok((output.count.0, width - 1, Cow::Owned(appended)))
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_text_splits_where_a_word_ends"),
        word: Word::Succeeds,
        input: || Scenario::Words,
        expect: Expect::Value((
            7,
            1,
            Cow::Borrowed(&[
                Cow::Borrowed("hello"),
                Cow::Borrowed("wor"),
                Cow::Borrowed("ld"),
                Cow::Borrowed("python"),
                Cow::Borrowed("provider"),
                Cow::Borrowed("v2"),
                Cow::Borrowed("beta"),
            ]),
        )),
    },
    Case {
        name: CaseName("a_text_without_letters_has_no_words"),
        word: Word::Succeeds,
        input: || Scenario::NoWords,
        expect: Expect::Value((0, 1, Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("words_require_their_real_provider"),
        word: Word::Refuses,
        input: || Scenario::MissingWords,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("only_matching_paths_remain"),
        word: Word::Succeeds,
        input: || Scenario::CaptureModule,
        expect: Expect::Value((1, 1, Cow::Borrowed(&[Cow::Borrowed("lib")]))),
    },
    Case {
        name: CaseName("a_starred_slot_takes_directories"),
        word: Word::Succeeds,
        input: || Scenario::CaptureStarred,
        expect: Expect::Value((
            1,
            3,
            Cow::Borrowed(&[
                Cow::Borrowed("a/b"),
                Cow::Borrowed("py"),
                Cow::Borrowed("src/x.rs"),
            ]),
        )),
    },
    Case {
        name: CaseName("every_way_a_pattern_matches_is_a_row"),
        word: Word::Succeeds,
        input: || Scenario::CaptureSplits,
        expect: Expect::Value((
            2,
            2,
            Cow::Borrowed(&[
                Cow::Borrowed("x"),
                Cow::Borrowed("y/z.rs"),
                Cow::Borrowed("x/y"),
                Cow::Borrowed("z.rs"),
            ]),
        )),
    },
    Case {
        name: CaseName("capture_requires_its_real_provider"),
        word: Word::Refuses,
        input: || Scenario::MissingCapture,
        expect: Expect::Error(OperatingError::NotRead),
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
