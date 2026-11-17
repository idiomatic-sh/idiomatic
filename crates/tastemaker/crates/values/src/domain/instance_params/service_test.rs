//! The `instance_params` service's cases, on values alone.

use std::collections::BTreeMap;

use super::fixtures::found;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::entries::fixtures::unwritten;
use crate::domain::entries::types::{ParamName, ParamSource, ParamValue, Value as EntryValue};
use crate::domain::scalars::types::TextValue;

/// What the spec entry passes its one param, `lang`, and where the spec takes it from when nothing passes it.
#[derive(Clone, Copy, Debug)]
enum Passed {
    /// Nothing passes it, and the spec takes the directory's name.
    DirectoryName,
    /// Nothing passes it, and the spec has no other source.
    Nothing,
    /// The entry passes a param its spec does not declare.
    Undeclared,
}

/// What the entry passes.
type Input = Passed;
/// The value `lang` is given, and the messages of the findings, joined.
type Value = (String, String);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_param_taken_from_the_directory_is_its_name"),
        word: Word::Succeeds,
        input: || Passed::DirectoryName,
        expect: Expect::Value((String::new(), String::new())),
    },
    Case {
        name: CaseName("a_param_nothing_gives_a_value_is_a_finding"),
        word: Word::Succeeds,
        input: || Passed::Nothing,
        expect: Expect::Value((String::new(), String::new())),
    },
    Case {
        name: CaseName("a_param_the_spec_does_not_declare_is_a_finding"),
        word: Word::Succeeds,
        input: || Passed::Undeclared,
        expect: Expect::Value((String::new(), String::new())),
    },
];

/// The value `lang` is given and the findings' messages, each against the one the row's case expects.
fn subject(passed: Input) -> Value {
    let mut given = BTreeMap::new();
    let (source, value, message) = match passed {
        Passed::DirectoryName => (ParamSource::DirectoryName, "toml", ""),
        Passed::Nothing => (
            ParamSource::Given,
            "",
            "`p/language` at `languages/toml` has no value for its param `lang`",
        ),
        Passed::Undeclared => {
            drop(given.insert(
                ParamName("extra".into()),
                ParamValue::Literal(EntryValue::Text(TextValue("x".into()))),
            ));
            (
                ParamSource::DirectoryName,
                "toml",
                "`p/language` passes `extra`, which its spec does not declare",
            )
        }
    };
    let (lang, said) = found(given, source, &unwritten);
    (
        if lang == value { String::new() } else { lang },
        if said == message { String::new() } else { said },
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
