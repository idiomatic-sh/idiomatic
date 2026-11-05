//! Literal replacement is distinct from capture interpolation.

use crate::application::file_dag::testing::TestOperator as _;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::replace::item::ReplaceItem;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::service::datum;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{CellKind, Relation, Value as Cell};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput, LiteralText};
use ryo_values::domain::plans::types::{Operation, QueryOperator};
use ryo_values::domain::rows::service::push_row;
use {
    crate::domain::table_texts::types::Datum, crate::domain::table_texts::types::Texts,
    ryo_values::domain::files::types::SourceView,
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
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(&[]), NameText(input.text));
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    push_row(&mut table, &[Cell::Symbol(symbol)]);
    let operation = Operation::Replace {
        from: PlanInput::Relation(Relation::Function),
        text: ColumnIndex(0),
        pattern: LiteralText(input.pattern.into()),
        replacement: LiteralText(input.replacement.into()),
    };
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    let mut output = Table::default();
    ReplaceItem.compute(
        &operation,
        &[Rows::Computed(&table)],
        &mut context,
        &mut output,
    )?;
    assert_eq!(output.count.0, 1);
    assert_eq!(output.cells[0], Cell::Symbol(symbol));
    let texts = Texts {
        symbols: context.symbols.as_ref(),
        source: context.source,
        paths: &[],
        arenas: &[],
    };
    let Datum::Text(text) = datum(output.cells[1], &texts) else {
        panic!("text output required")
    };
    assert_eq!(text.0, input.expected);
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
            selects(matches!(error, OperatingError::InvalidRegex { .. }))
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::Replace,
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;
