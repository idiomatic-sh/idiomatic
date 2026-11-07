//! Required row bindings are materialized only by the actual public request owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{LiteralText, ParameterId, TableKey},
        plans::types::Operation,
        query_bindings::types::{BindingCell, BindingTable, ParameterBinding},
    },
};
use std::borrow::Cow;
type Input = (&'static [&'static str], Option<ParameterBinding>);
type Value = Cow<'static, str>;
type Failure = OperatingError;
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

fn subject((keys, binding): Input) -> Result<Value, Failure> {
    let mut plan = fixtures::plan(vec![Operation::Rows {
        parameter: ParameterId(0),
        keys: keys.iter().map(|key| TableKey((*key).into())).collect(),
    }]);
    plan.parameters
        .push(ryo_values::domain::query_bindings::types::PlanParameter {
            name: ryo_values::domain::plan_parts::types::ParameterName("input".into()),
            kind: ryo_values::domain::query_bindings::types::ParameterType::Tables,
        });
    let mut symbols = Symbols::default();
    let output = fixtures::published(plan, binding.into_iter().collect(), vec![], &mut symbols)?;
    let rows: Vec<_> = output
        .cells
        .chunks(keys.len().max(1))
        .map(|row| {
            row.iter()
                .map(|cell| fixtures::text(*cell, &symbols))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    assert_eq!(rows.len(), output.count.0);
    Ok(Cow::Owned(rows.join("|")))
}
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
        expect: Expect::Value(Cow::Borrowed("a::Parse languages|b::Lint linter")),
    },
    Case {
        name: CaseName("no_tables_are_no_rows"),
        word: Word::Succeeds,
        input: || (&["path"], Some(tables(&[]))),
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("an_empty_list_of_any_kind_is_no_tables"),
        word: Word::Succeeds,
        input: || (&["path"], Some(ParameterBinding::Texts(Vec::new()))),
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a_table_without_a_key_asked_for_is_refused"),
        word: Word::Refuses,
        input: || (&["path"], Some(tables(&[&[("resource", "languages")]]))),
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("a_rows_parameter_nothing_bound_is_refused"),
        word: Word::Refuses,
        input: || (&["path"], None),
        expect: Expect::Error(OperatingError::Unbound),
    },
    Case {
        name: CaseName("a_rows_parameter_bound_to_a_text_is_refused"),
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
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
