//! Catalog availability is independent of occurrences, embeddings and consumer target versions.
use super::super::fixtures::contract::check;
use super::item::ImplicitSymbolItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, proposed};
use ryo::languages::contract::Want;
use std::collections::BTreeSet;
type Input = &'static str;
type Value = &'static str;
type Failure = ();
const EXPECTED: &str = "171|builtin:ExceptionGroup:3.11|builtin:__doc__:3.0|builtin:__loader__:3.0|builtin:__name__:3.0|builtin:__package__:3.0|builtin:__spec__:3.4|builtin:aiter:3.10|builtin:anext:3.10|builtin:len:3.0|module-global:__annotate__:3.14|module-global:__annotations__:3.6|module-global:__builtins__:3.0|module-global:__cached__:3.2|module-global:__file__:3.0|module-global:__name__:3.0|module-global:__package__:3.0|module-global:__spec__:3.4|module-global:__warningregistry__:3.0|platform-builtin:WindowsError:3.0";
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty physical module still supplies implicit namespace catalog"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(EXPECTED),
    },
    Case {
        name: CaseName("string literal has no bearing on catalog availability"),
        word: Word::Succeeds,
        input: || "'len'\n",
        expect: Expect::Value(EXPECTED),
    },
    Case {
        name: CaseName("embedded expressions do not duplicate module catalog"),
        word: Word::Succeeds,
        input: || "first: 'len'\nsecond: 'ExceptionGroup'\nthird: 'aiter'\n",
        expect: Expect::Value(EXPECTED),
    },
    Case {
        name: CaseName("source diagnostics do not erase known catalog"),
        word: Word::Succeeds,
        input: || "if:\n    pass\n",
        expect: Expect::Value(EXPECTED),
    },
];
fn subject(source: Input) -> Result<Value, Failure> {
    check::<ImplicitSymbolItem>();
    let answer = proposed::<ImplicitSymbolItem>(source);
    if !answer.causes.is_empty() {
        return Err(());
    }
    let mut names = BTreeSet::new();
    let mut observed = vec![answer.rows.len().to_string()];
    let mut module_scope = None;
    for row in &answer.rows {
        let [
            Want::Name(name),
            Want::Name(kind),
            Want::Node(scope),
            Want::Number(major),
            Want::Number(minor),
            Want::File(file),
        ] = **row
        else {
            return Err(());
        };
        if *file != SHOWN {
            return Err(());
        }
        if let Some(expected_scope) = module_scope {
            assert_eq!(
                *scope, expected_scope,
                "catalog scope is the physical module root"
            );
        } else {
            module_scope = Some(*scope);
        }
        assert!(
            names.insert((*name, *kind)),
            "one row per name and origin in the physical module"
        );
        for (wanted_kind, wanted_name) in [
            ("builtin", "len"),
            ("builtin", "ExceptionGroup"),
            ("builtin", "aiter"),
            ("builtin", "anext"),
            ("builtin", "__name__"),
            ("builtin", "__doc__"),
            ("builtin", "__loader__"),
            ("builtin", "__package__"),
            ("builtin", "__spec__"),
            ("module-global", "__name__"),
            ("module-global", "__annotations__"),
            ("module-global", "__builtins__"),
            ("module-global", "__cached__"),
            ("module-global", "__file__"),
            ("module-global", "__annotate__"),
            ("module-global", "__package__"),
            ("module-global", "__spec__"),
            ("module-global", "__warningregistry__"),
            ("platform-builtin", "WindowsError"),
        ] {
            if *kind == wanted_kind && *name == wanted_name {
                observed.push(format!("{kind}:{name}:{major}.{minor}"));
            }
        }
    }
    let count = observed.remove(0);
    observed.sort_unstable();
    observed.insert(0, count);
    Ok(Box::leak(observed.join("|").into_boxed_str()))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
