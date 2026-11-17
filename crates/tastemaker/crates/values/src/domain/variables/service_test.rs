//! What the `variables` service puts in place, and what it refuses.

use super::service::{resolve, selector_of};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::entries::types::{
    Alias, Composition, Described, EntryHeader, ParamName, ParamValue, ParamValues, RuleEntry,
    Value as EntryValue, Variables,
};
use crate::domain::packages::types::{DefinitionName, Ownership, PackageName, VersionText};
use crate::domain::scalars::types::{TextValue, ValueKey};
use crate::domain::selectors::types::{Selector, Selectors, VariableName};
use crate::domain::variables::types::VariableProblem;

/// The variables declared and the references an entry makes.
type Input = (&'static [&'static str], &'static [&'static str]);
/// How many selectors the entry's `exclude` holds after.
type Value = usize;
type Failure = VariableProblem;

/// The variable a case's `imports-allow` names.
const ALLOWED: &str = "allowed";

fn list() -> EntryValue {
    EntryValue::List(vec![
        EntryValue::Text(TextValue("**/tests/**".into())),
        EntryValue::Text(TextValue("**/*_test.txt".into())),
    ])
}

fn composition(declared: &[&str], referred: &[&str]) -> Composition {
    let mut variables = Variables::default();
    for name in declared {
        drop(variables.0.insert(VariableName((*name).into()), list()));
    }
    let header = EntryHeader {
        package: PackageName("p".into()),
        version: VersionText("0.1.0".into()),
        ownership: Ownership::Shared,
        alias: Alias::Unnamed,
        description: Described::Unsaid,
        include: Selectors(vec![Selector::Glob(
            crate::domain::selectors::types::PathPattern("**".into()),
        )]),
        exclude: Selectors(
            referred
                .iter()
                .map(|name| Selector::Variable(VariableName((*name).into())))
                .collect(),
        ),
        params: ParamValues(
            referred
                .iter()
                .map(|name| {
                    (
                        ParamName("libraries".into()),
                        ParamValue::Variable(VariableName((*name).into())),
                    )
                })
                .collect(),
        ),
    };
    // A variable named `allowed` is the file's `imports-allow`.
    let imports_allow_variables = if declared.contains(&ALLOWED) {
        vec![VariableName(ALLOWED.into())]
    } else {
        Vec::new()
    };
    Composition {
        rules: vec![RuleEntry {
            header,
            rule: DefinitionName("r".into()),
        }],
        variables,
        imports_allow_variables,
        ..Composition::default()
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_list_variable_splices_into_a_selector_list"),
        word: Word::Succeeds,
        input: || (&["tests"], &["tests"]),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("an_imports_allow_variable_is_put_in_place_and_used"),
        word: Word::Succeeds,
        input: || (&["tests", ALLOWED], &["tests"]),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("an_undefined_variable_is_refused"),
        word: Word::Refuses,
        input: || (&[], &["tests"]),
        expect: Expect::Refusal(|problem| {
            if matches!(problem, VariableProblem::Undefined(_)) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_unused_variable_is_refused"),
        word: Word::Refuses,
        input: || (&["tests", "spare"], &["tests"]),
        expect: Expect::Refusal(|problem| {
            if matches!(problem, VariableProblem::Unused(_)) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

fn subject((declared, referred): Input) -> Result<Value, Failure> {
    let mut composition = composition(declared, referred);
    if let Some(problem) = resolve(&mut composition, &mut []).into_iter().next() {
        return Err(problem);
    }
    if declared.contains(&ALLOWED) {
        assert_eq!(composition.imports_allow.len(), 2);
    }
    let entry = &composition.rules[0];
    assert!(matches!(
        entry.header.params.0.get(&ParamName("libraries".into())),
        Some(ParamValue::Literal(EntryValue::List(_)))
    ));
    Ok(entry.header.exclude.0.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    let mut table = std::collections::BTreeMap::new();
    drop(table.insert(
        ValueKey("part".into()),
        EntryValue::Text(TextValue("use-case".into())),
    ));
    assert!(matches!(
        selector_of(&EntryValue::Table(table)),
        Some(Selector::Part(_))
    ));
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
