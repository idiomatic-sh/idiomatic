//! The structure cases of the parent test file.

use super::{Failure, Input, Value, subject};
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("redundant parentheses and whitespace are not expression fields"),
        word: Word::Succeeds,
        input: || ("(a + b)\na+b\n", "a + b", "a+b"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("operand order remains significant"),
        word: Word::Succeeds,
        input: || ("a+b\nb+a\n", "a+b", "b+a"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("operator remains significant"),
        word: Word::Succeeds,
        input: || ("a+b\na-b\n", "a+b", "a-b"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("read and write contexts are ignored"),
        word: Word::Succeeds,
        input: || ("x=0\nx\n", "x", "x"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("NFKC identifiers share lexical identity"),
        word: Word::Succeeds,
        input: || ("𝗲𝘅𝗲𝗰\nexec\n", "𝗲𝘅𝗲𝗰", "exec"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("small integer radix is normalized"),
        word: Word::Succeeds,
        input: || ("16\n0x10\n", "16", "0x10"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("integer and floating literal kinds are distinct"),
        word: Word::Succeeds,
        input: || ("1\n1.0\n", "1", "1.0"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("boolean is not an integer shape"),
        word: Word::Succeeds,
        input: || ("True\n1\n", "True", "1"),
        expect: Expect::Value(false),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
