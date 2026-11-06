//! The dynamic names cases of the parent test file.

use super::{Failure, Input, Value, subject};
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordinary_dynamic"),
        word: Word::Succeeds,
        input: || ("def f(x):\n    x()\n", "x"),
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("mixed_dynamic_and_known"),
        word: Word::Succeeds,
        input: || {
            (
                "def f(x):\n    x()\nimport contextlib\ncontextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("receiver_call_is_dynamic"),
        word: Word::Succeeds,
        input: || ("factory().suppress()\n", "factory().suppress"),
        expect: Expect::Value((None, "unresolved", 0, 0)),
    },
    Case {
        name: CaseName("global_binding"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\ndef f():\n    global contextlib\n    contextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("comprehension_local"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\n[contextlib.suppress() for contextlib in values]\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("comprehension_outer_iterable"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\n[x for contextlib in contextlib.suppress()]\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
