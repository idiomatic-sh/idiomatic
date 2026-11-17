//! A literal param that names a capture takes the capture's value, written as the manifest adapter writes it.

use std::collections::BTreeMap;

use crate::cases::{Case, CaseName, Expect, Word, run};
use tastemaker_values::domain::entries::types::{ParamName, ParamValue, ParamValues, Value as EntryValue};
use crate::domain::scaffolding::fixtures::captures;
use crate::domain::scaffolding::service::literal_values;
use tastemaker_values::domain::scalars::types::TextValue;

/// The literal an entry passes `trait`, with `lang = go`.
type Input = &'static str;
/// The values it gives, empty when they are the ones expected.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_literal_param_takes_the_captures"),
    word: Word::Succeeds,
    input: || "pkg::{lang}",
    expect: Expect::Value(String::new()),
}];

fn subject(literal: Input) -> Value {
    let passed = ParamValues(BTreeMap::from([(
        ParamName("trait".into()),
        ParamValue::Literal(EntryValue::Text(TextValue(literal.into()))),
    )]));
    let said = format!(
        "{:?}",
        literal_values(
            &passed,
            &captures(&[("lang", "go")]),
            &crate::outbound::manifest::factory::toml()
        )
    );
    if said == format!("{:?}", captures(&[("trait", "pkg::go")])) {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
