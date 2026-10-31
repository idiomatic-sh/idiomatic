//! Catalog availability is independent of occurrences, embeddings and the consumer's target version.
use super::super::fixtures::contract::check;
use super::item::ImplicitSymbolItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, proposed};
use ryo::languages::contract::Want;
type Input = &'static str;
type Value = &'static str;
type Failure = ();
const EXPECTED: &str = "156|len:3.0|ExceptionGroup:3.11|aiter:3.10|anext:3.10|module-global:absent";
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty physical module still supplies builtin inventory"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(EXPECTED),
    },
    Case {
        name: CaseName("string literal does not need a Name occurrence for catalog lookup"),
        word: Word::Succeeds,
        input: || "'len'\n",
        expect: Expect::Value(EXPECTED),
    },
    Case {
        name: CaseName(
            "multiple embedded expressions cannot duplicate the physical module inventory",
        ),
        word: Word::Succeeds,
        input: || "first: 'len'\nsecond: 'ExceptionGroup'\nthird: 'aiter'\n",
        expect: Expect::Value(EXPECTED),
    },
    Case {
        name: CaseName("source errors do not change the known provider language catalog"),
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
    let mut names = Vec::new();
    for row in answer.rows {
        let [
            Want::Name(name),
            Want::Number(_),
            Want::Number(_),
            Want::File(file),
        ] = *row
        else {
            return Err(());
        };
        if *file != SHOWN {
            return Err(());
        }
        names.push(*name);
    }
    names.sort_unstable();
    names.dedup();
    assert_eq!(
        names.len(),
        answer.rows.len(),
        "one physical module emits each catalog name once"
    );
    let mut observed = vec![answer.rows.len().to_string()];
    for wanted in ["len", "ExceptionGroup", "aiter", "anext"] {
        let (major, minor) = answer
            .rows
            .iter()
            .find_map(|row| match *row {
                [
                    Want::Name(name),
                    Want::Number(major),
                    Want::Number(minor),
                    _,
                ] if *name == wanted => Some((*major, *minor)),
                _ => None,
            })
            .ok_or(())?;
        observed.push(format!("{wanted}:{major}.{minor}"));
    }
    assert!(
        !names.contains(&"__file__") && !names.contains(&"__name__"),
        "implicit module globals are not builtin symbols"
    );
    observed.push("module-global:absent".to_owned());
    Ok(Box::leak(observed.join("|").into_boxed_str()))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
