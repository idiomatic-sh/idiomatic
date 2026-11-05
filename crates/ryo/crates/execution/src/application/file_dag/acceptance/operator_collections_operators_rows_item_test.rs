//! The `rows` item's cases.

use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::Value as Cell;

use crate::application::file_dag::testing::TestOperator as _;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::rows::item::RowsItem;
use crate::domain::table_model::types::{};
use crate::domain::table_scratch::types::OperatorScratch;
use {ryo_values::domain::files::types::SourceView};
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::{ParameterId, TableKey};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::QueryOperator;

/// The texts of each row, in order: a row's texts apart by a space, the rows by a bar.
type Value = &'static str;
type Failure = OperatingError;
/// A scenario: the keys asked for, and what the parameter is bound to.
type Input = (&'static [&'static str], Option<ParameterBinding>);

/// The tables, each a list of its key and text pairs.
fn tables(rows: &[&[(&str, &str)]]) -> ParameterBinding {
    ParameterBinding::Tables(
        rows.iter()
            .map(|cells| {
                BindingTable(
                    cells
                        .iter()
                        .map(|(key, value)| BindingCell {
                            key: LiteralText((*key).into()),
                            value: LiteralText((*value).into()),
                        })
                        .collect(),
                )
            })
            .collect(),
    )
}

/// The texts of the rows the item makes of the parameter, read back from the names it made.
fn subject((keys, binding): Input) -> Result<Value, Failure> {
    let operation = Operation::Rows {
        parameter: ParameterId(0),
        keys: keys.iter().map(|key| TableKey((*key).into())).collect(),
    };
    let bound: Vec<ParameterBinding> = binding.into_iter().collect();
    let mut symbols = Symbols::default();
    let mut out = Table::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &bound,
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    RowsItem.compute(&operation, &[], &mut context, &mut out)?;
    let said = out
        .cells
        .chunks(keys.len().max(1))
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    Cell::Symbol(symbol) => text_of(&symbols, ByteSlice(&[]), *symbol)
                        .map(|text| text.0.to_owned())
                        .unwrap_or_default(),
                    Cell::Node(_)
                    | Cell::Name(_)
                    | Cell::File(_)
                    | Cell::Span(_)
                    | Cell::Number(_)
                    | Cell::Flag(_) => String::new(),
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("|");
    Ok(Box::leak(said.into_boxed_str()))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_table_is_a_row_of_the_texts_under_the_keys_in_order"),
        word: Word::Succeeds,
        input: || {
            (
                &["path", "resource"],
                Some(tables(&[
                    &[
                        ("resource", "languages"),
                        ("path", "a::Parse"),
                        ("as", "Parse"),
                    ],
                    &[("resource", "linter"), ("path", "b::Lint"), ("as", "Lint")],
                ])),
            )
        },
        expect: Expect::Value("a::Parse languages|b::Lint linter"),
    },
    Case {
        name: CaseName("no_tables_are_no_rows"),
        word: Word::Succeeds,
        input: || (&["path"], Some(tables(&[]))),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("an_empty_list_of_any_kind_is_no_tables"),
        word: Word::Succeeds,
        input: || (&["path"], Some(ParameterBinding::Texts(Vec::new()))),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_table_without_a_key_asked_for_is_refused"),
        word: Word::Refuses,
        input: || (&["path"], Some(tables(&[&[("resource", "languages")]]))),
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("a_parameter_nothing_bound_is_refused"),
        word: Word::Refuses,
        input: || (&["path"], None),
        expect: Expect::Error(OperatingError::Unbound),
    },
    Case {
        name: CaseName("a_parameter_bound_to_a_text_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                &["path"],
                Some(ParameterBinding::Text(LiteralText("path".into()))),
            )
        },
        expect: Expect::Error(OperatingError::WrongBinding),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(QueryOperator::Rows);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;

use ryo_values::domain::query_bindings::types::{ParameterBinding,BindingTable,BindingCell};
