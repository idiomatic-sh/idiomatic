//! The strings and calls cases of the parent test file.

use super::{Failure, Input, Value, subject};
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("implicit string concatenation is one decoded literal"),
        word: Word::Succeeds,
        input: || ("'a' 'b'\n'ab'\n", "'a' 'b'", "'ab'"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("bytes and text are distinct literal kinds"),
        word: Word::Succeeds,
        input: || ("b'ab'\n'ab'\n", "b'ab'", "'ab'"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("call positional and keyword fields are grouped independently"),
        word: Word::Succeeds,
        input: || ("f(k=1,*xs)\nf(*xs,k=1)\n", "f(k=1,*xs)", "f(*xs,k=1)"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("keyword order is preserved within the keyword field"),
        word: Word::Succeeds,
        input: || ("f(a=1,b=2)\nf(b=2,a=1)\n", "f(a=1,b=2)", "f(b=2,a=1)"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("dictionary unpack positions are significant"),
        word: Word::Succeeds,
        input: || ("{'a':x,**y}\n{**x,'a':y}\n", "{'a':x,**y}", "{**x,'a':y}"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("slice omitted fields do not collapse"),
        word: Word::Succeeds,
        input: || ("x[:a]\nx[a:]\n", "x[:a]", "x[a:]"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("formatted string conversion is significant"),
        word: Word::Succeeds,
        input: || ("f'{x!s}'\nf'{x!r}'\n", "f'{x!s}'", "f'{x!r}'"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("formatted string empty format is not absent format"),
        word: Word::Succeeds,
        input: || ("f'{x}'\nf'{x:}'\n", "f'{x}'", "f'{x:}'"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("formatted string debug source contributes to its value"),
        word: Word::Succeeds,
        input: || ("f'{x=}'\nf'{x =}'\n", "f'{x=}'", "f'{x =}'"),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("formatted string adjacent literal segments merge"),
        word: Word::Succeeds,
        input: || ("f'{x}a' 'b'\nf'{x}ab'\n", "f'{x}a' 'b'", "f'{x}ab'"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("f-string escape spellings share decoded literal text"),
        word: Word::Succeeds,
        input: || ("f'{x}\\n'\nf'{x}\\x0a'\n", "f'{x}\\n'", "f'{x}\\x0a'"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("f-string escaped braces share decoded literal text"),
        word: Word::Succeeds,
        input: || {
            (
                "f'{{{x}}}'\nf'\\x7b{x}\\x7d'\n",
                "f'{{{x}}}'",
                "f'\\x7b{x}\\x7d'",
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unequal f-string literal fragments remain distinct"),
        word: Word::Succeeds,
        input: || ("f'{x}a'\nf'{x}b'\n", "f'{x}a'", "f'{x}b'"),
        expect: Expect::Value(false),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
