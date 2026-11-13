//! The semantic identity cases of the parent test file.

use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;

/// What a row is given: the parent table's input.
type Input = super::Input;
/// What a row expects: the parent table's value.
type Value = super::Value;
/// What a refused row expects: the parent table's failure.
type Failure = super::Failure;

/// Runs one row through the parent table's subject.
fn subject(input: Input) -> Result<Value, Failure> {
    super::subject(input)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("decoded byte escapes normalize"),
        word: Word::Succeeds,
        input: || ("b\"a\"\nb\"\\x61\"\n", "b\"a\"", "b\"\\x61\""),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("semantic Name in a simple annotation matches native identity"),
        word: Word::Succeeds,
        input: || ("x\nvalue: \"x\"\n", "x", "x"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("nested semantic expression ignores formatting"),
        word: Word::Succeeds,
        input: || ("a+b\nvalue: \"a + b\"\n", "a+b", "a + b"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("lambda parameter categories preserve positional only separator"),
        word: Word::Succeeds,
        input: || {
            (
                "lambda x: x\nlambda x, /: x\n",
                "lambda x: x",
                "lambda x, /: x",
            )
        },
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("comprehension async flag is structural"),
        word: Word::Succeeds,
        input: || {
            (
                "[x for x in xs]\n[x async for x in xs]\n",
                "[x for x in xs]",
                "[x async for x in xs]",
            )
        },
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("valid invalid empty valid reparses cannot leak classes"),
        word: Word::Succeeds,
        input: || ("reparse witness", "", ""),
        expect: Expect::Value(true),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
