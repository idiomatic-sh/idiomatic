//! The semantic identity cases of the parent test file.

use super::{Failure, Input, Value, subject};
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;

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
        name: CaseName("full key comparison survives deliberately colliding hashes"),
        word: Word::Succeeds,
        input: || ("collision witness", "", ""),
        expect: Expect::Value(true),
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
