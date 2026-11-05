//! The `words` item's cases.

use ryo_language::domain::symbols::service::{intern, text_of};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Offset, Relation, Value as Cell};
use ryo_values::domain::plans::types::QueryOperator;

use crate::application::file_dag::testing::TestOperator as _;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::words::item::WordsItem;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::service::push_row;

type Value = usize;
type Failure = OperatingError;
/// A scenario: what a row computes with the item, answering how many rows came out.
type Input = fn() -> Result<Value, OperatingError>;

const FROM: PlanInput = PlanInput::Relation(Relation::Comment);

/// The words the text splits into, as the item says them.
fn words_of(source: &str) -> Result<Vec<String>, OperatingError> {
    let mut symbols = Symbols::default();
    let bytes = source.as_bytes();
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    push_row(
        &mut table,
        &[Cell::Symbol(intern(
            &mut symbols,
            ByteSlice(bytes),
            Offset(0),
            Offset(u32::try_from(bytes.len()).unwrap_or(u32::MAX)),
        ))],
    );
    let operation = Operation::Words {
        from: FROM,
        text: ColumnIndex(0),
    };
    let mut out = Table::default();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(bytes),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    WordsItem.compute(
        &operation,
        &[Rows::Computed(&table)],
        &mut context,
        &mut out,
    )?;
    Ok(out
        .cells
        .chunks(2)
        .filter_map(|row| {
            if let Cell::Symbol(word) = row[1] {
                text_of(&symbols, ByteSlice(bytes), word).map(|text| text.0.to_owned())
            } else {
                None
            }
        })
        .collect())
}

fn splits_where_a_word_ends() -> Result<Value, OperatingError> {
    Ok(usize::from(
        words_of("Hello wor_ld, PythonProvider v2Beta")?
            == ["hello", "wor", "ld", "python", "provider", "v2", "beta"],
    ))
}

fn a_text_without_letters_has_no_words() -> Result<Value, OperatingError> {
    Ok(words_of(" -- ... ")?.len())
}

fn words_without_an_input() -> Result<Value, OperatingError> {
    let mut symbols = Symbols::default();
    let mut out = Table::default();
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
    WordsItem
        .compute(
            &Operation::Words {
                from: FROM,
                text: ColumnIndex(0),
            },
            &[],
            &mut context,
            &mut out,
        )
        .map(|()| out.count.0)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_text_splits_where_a_word_ends"),
        word: Word::Succeeds,
        input: || splits_where_a_word_ends,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_text_without_letters_has_no_words"),
        word: Word::Succeeds,
        input: || a_text_without_letters_has_no_words,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_missing_input_is_refused"),
        word: Word::Refuses,
        input: || words_without_an_input,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, OperatingError::MissingInput { .. }))
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::Words,
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;
