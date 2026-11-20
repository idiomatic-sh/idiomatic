//! The `terminal_facts` service's cases: how the functions of a source end.

use super::service::outcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::node::service::typed_nodes;
use crate::fixtures::syntax_files::with_file;
use ruff_python_ast::AnyNodeRef;

/// A source.
type Input = &'static str;
/// The outcome of each function of the source, in the order they are written, joined by `|`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_body_that_falls_off_its_end_is_implicit_and_one_that_returns_is_not"),
        word: Word::Succeeds,
        input: || "def f():\n    pass\ndef g():\n    return 1\ndef h():\n    raise E\n",
        expect: Expect::Value("implicit|return-or-raise|raise"),
    },
    Case {
        name: CaseName("a_return_on_one_path_only_is_conditional"),
        word: Word::Succeeds,
        input: || "def f(a):\n    if a:\n        return 1\ndef g(a):\n    if a:\n        return 1\n    else:\n        raise E\n",
        expect: Expect::Value("conditional-return|return-or-raise"),
    },
    Case {
        name: CaseName("a_loop_that_always_breaks_ends_nothing_and_a_nested_definition_is_its_own"),
        word: Word::Succeeds,
        input: || "def f(a):\n    while a:\n        break\n        return 1\n    def g():\n        return 2\n",
        expect: Expect::Value("implicit|return-or-raise"),
    },
    Case {
        name: CaseName("a_match_with_a_case_that_takes_all_is_exhaustive"),
        word: Word::Succeeds,
        input: || "def f(a):\n    match a:\n        case 1:\n            return 1\n        case _:\n            return 2\ndef g(a):\n    match a:\n        case 1:\n            return 1\n",
        expect: Expect::Value("return-or-raise|conditional-return"),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    let mut shown: Vec<&'static str> = Vec::new();
    with_file(source, &mut |file| {
        for numbered in typed_nodes(file) {
            if let AnyNodeRef::StmtFunctionDef(function) = numbered.node {
                shown.push(outcome(function).0);
            }
        }
    });
    Ok(Box::leak(shown.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
