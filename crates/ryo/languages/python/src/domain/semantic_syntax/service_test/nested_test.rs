//! Embedded expressions nested in embedded expressions take their holder's visibility.
use super::{Failure, Input, Value, subject};
use crate::cases::{Case, CaseName, Expect, Word, run};

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
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
