//! Embedded expressions nested in embedded expressions take their holder's visibility.
use crate::cases::{Case, CaseName, Expect, Word, run};

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

/// Each expectation is fixed independently of the parser and relation implementation.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nested quoted operand of a visible generic is visible"),
        word: Word::Succeeds,
        input: || Input::Source("import alpha\nx: \"list['alpha.run()']\"\n"),
        expect: Expect::Value((1, "alpha.run", 23, 34)),
    },
    Case {
        name: CaseName("sibling nested quoted operands are each visible"),
        word: Word::Succeeds,
        input: || Input::Source("import alpha\nx: \"dict['alpha.run()', 'alpha.run()']\"\n"),
        expect: Expect::Value((2, "alpha.run|alpha.run", 23, 34)),
    },
    Case {
        name: CaseName("nested quoted Literal argument is a value"),
        word: Word::Succeeds,
        input: || {
            Input::Source(
                "from typing import Literal\nimport alpha\nx: \"Literal['alpha.run()']\"\n",
            )
        },
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName("nested quoted operand inside a hidden value stays hidden"),
        word: Word::Succeeds,
        input: || {
            Input::Source(
                "from typing import cast\nimport alpha\nx = cast(int, \"list['alpha.run()']\")\n",
            )
        },
        expect: Expect::Value((0, "", 0, 0)),
    },
    Case {
        name: CaseName("a cast operand nested blocks deep is still a type"),
        word: Word::Succeeds,
        input: || {
            Input::Source(
                "from typing import cast\nimport alpha\nif a:\n    if b:\n        x = cast(\"alpha.run()\", v)\n",
            )
        },
        expect: Expect::Value((1, "alpha.run", 71, 82)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
