//! Function terminal summaries cover structured statement paths.
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::super::fixtures::contract::check;
use super::item::FunctionTerminalItem;
use crate::fixtures::reading::reading;

use ryo::languages::contract::Want;

fn classifies_implicit_raise_return_and_conditional_paths() {
    check::<FunctionTerminalItem>();
    let report = reading::<FunctionTerminalItem>(concat!(
        "def implicit():\n    pass\n",
        "def raises():\n    raise ValueError()\n",
        "def returns(flag):\n    if flag:\n        return 1\n    else:\n        raise ValueError()\n",
        "def conditional(flag):\n    if flag:\n        return 1\n",
        "class C:\n    def nested():\n        return None\n",
        "def handled():\n    try:\n        raise ValueError()\n    except ValueError:\n        pass\n",
        "def loop_break():\n    while True:\n        break\n",
        "def matched(value):\n    match value:\n        case _:\n            return 1\n",
        "async def async_return():\n    return 1\n",
    ));
    assert!(report.causes.is_empty(), "{:?}", report.causes);
    assert_eq!(report.rows.len(), 9);
    let outcomes = report.rows.iter().map(|row| row[1]).collect::<Vec<_>>();
    assert_eq!(
        outcomes,
        [
            Want::Name("implicit"),
            Want::Name("raise"),
            Want::Name("return-or-raise"),
            Want::Name("conditional-return"),
            Want::Name("return-or-raise"),
            Want::Name("implicit"),
            Want::Name("implicit"),
            Want::Name("return-or-raise"),
            Want::Name("return-or-raise"),
        ]
    );
}

fn parser_recovery_does_not_certify_function_outcomes() {
    check::<FunctionTerminalItem>();
    let report = reading::<FunctionTerminalItem>("def f():\n    return (\n");
    assert!(!report.causes.is_empty());
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("classifies_implicit_raise_return_and_conditional_paths"),
        word: Word::Succeeds,
        input: || classifies_implicit_raise_return_and_conditional_paths,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("parser_recovery_does_not_certify_function_outcomes"),
        word: Word::Succeeds,
        input: || parser_recovery_does_not_certify_function_outcomes,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
