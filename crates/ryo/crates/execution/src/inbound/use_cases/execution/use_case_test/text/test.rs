//! Text row operators retain the original row and publish exact appended words or slots.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
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
/// The text operator a row runs: words, or a capture of a path pattern.
#[derive(Clone, Copy, Debug)]
enum Operator {
    Words,
    Capture(&'static str),
}

/// The hosted texts the operator reads, or no hosted provider, so that it reads the function relation.
#[derive(Clone, Copy, Debug)]
struct Texts {
    originals: Option<&'static [&'static str]>,
    operator: Operator,
}

/// What the operator publishes: its rows, the slots it appends to each, every appended text in
/// order, and whether every row keeps an original text in its first column.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Appended {
    rows: usize,
    slots: usize,
    texts: Cow<'static, [Cow<'static, str>]>,
    originals_kept: bool,
}

type Input = Texts;
type Value = Appended;
type Failure = OperatingError;

/// A pattern of two starred slots.
const SPLIT: &str = "{*ancestor}/{*rest}";

/// Runs the operator over the texts and publishes its rows.
fn subject(texts: Input) -> Result<Value, Failure> {
    let missing = texts.originals.is_none();
    let originals = texts.originals.unwrap_or_default();
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
    operations.push(match texts.operator {
        Operator::Words => Operation::Words { from, text },
        Operator::Capture(pattern) => Operation::Capture {
            from,
            path: text,
            pattern: CapturePattern(pattern.into()),
        },
    });
    let mut symbols = Symbols::default();
    let output =
        fixtures::requests::published(inputs::plan(operations), vec![], hosted, &mut symbols)?;
    let width = output.kinds.len();
    let mut appended = Vec::new();
    let mut originals_kept = output.cells.len() == output.count.0 * width;
    for row in output.cells.chunks(width) {
        originals_kept &= originals.contains(&inputs::text(row[0], &symbols));
        appended.extend(
            row[1..]
                .iter()
                .map(|cell| Cow::Owned(inputs::text(*cell, &symbols).to_owned())),
        );
    }
    Ok(Appended {
        rows: output.count.0,
        slots: width - 1,
        texts: Cow::Owned(appended),
        originals_kept,
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_text_splits_where_a_word_ends"),
        word: Word::Succeeds,
        input: || Texts {
            originals: Some(&["Hello wor_ld, PythonProvider v2Beta"]),
            operator: Operator::Words,
        },
        expect: Expect::Value(Appended {
            rows: 7,
            slots: 1,
            texts: Cow::Borrowed(&[
                Cow::Borrowed("hello"),
                Cow::Borrowed("wor"),
                Cow::Borrowed("ld"),
                Cow::Borrowed("python"),
                Cow::Borrowed("provider"),
                Cow::Borrowed("v2"),
                Cow::Borrowed("beta"),
            ]),
            originals_kept: true,
        }),
    },
    Case {
        name: CaseName("a_text_without_letters_has_no_words"),
        word: Word::Succeeds,
        input: || Texts {
            originals: Some(&[" -- ... "]),
            operator: Operator::Words,
        },
        expect: Expect::Value(Appended {
            rows: 0,
            slots: 1,
            texts: Cow::Borrowed(&[]),
            originals_kept: true,
        }),
    },
    Case {
        name: CaseName("words_require_their_real_provider"),
        word: Word::Refuses,
        input: || Texts {
            originals: None,
            operator: Operator::Words,
        },
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("only_matching_paths_remain"),
        word: Word::Succeeds,
        input: || Texts {
            originals: Some(&["src/lib.rs", "src/deep/mod.rs", "README.md"]),
            operator: Operator::Capture("src/{module}.rs"),
        },
        expect: Expect::Value(Appended {
            rows: 1,
            slots: 1,
            texts: Cow::Borrowed(&[Cow::Borrowed("lib")]),
            originals_kept: true,
        }),
    },
    Case {
        name: CaseName("a_starred_slot_takes_directories"),
        word: Word::Succeeds,
        input: || Texts {
            originals: Some(&["a/b/languages/py/src/x.rs", "languages/py/x.r", ""]),
            operator: Operator::Capture("{*above}/languages/{name}/{*below}"),
        },
        expect: Expect::Value(Appended {
            rows: 1,
            slots: 3,
            texts: Cow::Borrowed(&[
                Cow::Borrowed("a/b"),
                Cow::Borrowed("py"),
                Cow::Borrowed("src/x.rs"),
            ]),
            originals_kept: true,
        }),
    },
    Case {
        name: CaseName("every_way_a_pattern_matches_is_a_row"),
        word: Word::Succeeds,
        input: || Texts {
            originals: Some(&["x/y/z.rs"]),
            operator: Operator::Capture(SPLIT),
        },
        expect: Expect::Value(Appended {
            rows: 2,
            slots: 2,
            texts: Cow::Borrowed(&[
                Cow::Borrowed("x"),
                Cow::Borrowed("y/z.rs"),
                Cow::Borrowed("x/y"),
                Cow::Borrowed("z.rs"),
            ]),
            originals_kept: true,
        }),
    },
    Case {
        name: CaseName("capture_requires_its_real_provider"),
        word: Word::Refuses,
        input: || Texts {
            originals: None,
            operator: Operator::Capture(SPLIT),
        },
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
