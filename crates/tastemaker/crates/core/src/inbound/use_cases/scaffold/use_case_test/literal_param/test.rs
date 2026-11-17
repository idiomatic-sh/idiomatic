//! A literal an entry passes is the param's value, written as the manifest adapter writes it.

use std::collections::BTreeMap;

use crate::cases::{Case, CaseName, Expect, Word, run};
use tastemaker_values::domain::entries::types::{ParamName, ParamSource, ParamValue, Value as EntryValue};
use tastemaker_values::domain::instance_params::fixtures::found;
use tastemaker_values::domain::scalars::types::TextValue;

/// The literal the entry passes `lang`.
type Input = &'static str;
/// The value `lang` is given and the messages of the findings, each empty when it is the one expected.
type Value = (String, String);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_literal_the_entry_passes_is_the_value"),
    word: Word::Succeeds,
    input: || "rust",
    expect: Expect::Value((String::new(), String::new())),
}];

fn subject(literal: Input) -> Value {
    let given = BTreeMap::from([(
        ParamName("lang".into()),
        ParamValue::Literal(EntryValue::Text(TextValue(literal.into()))),
    )]);
    let (lang, said) = found(
        given,
        ParamSource::Given,
        &crate::outbound::manifest::factory::toml(),
    );
    (if lang == literal { String::new() } else { lang }, said)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
