//! The builtin names cases of the parent test file.

use super::{Failure, Input, Value, subject};
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("builtin"),
        word: Word::Succeeds,
        input: || ("exec(source)\n", "exec"),
        expect: Expect::Value((Some("builtins.exec"), "builtin", 3, 0)),
    },
    Case {
        name: CaseName("module-global __name__ is not a builtins fallback"),
        word: Word::Succeeds,
        input: || ("__name__\n", "__name__"),
        expect: Expect::Value((None, "unresolved", 0, 0)),
    },
    Case {
        name: CaseName("explicit builtins __name__ keeps its qualified origin"),
        word: Word::Succeeds,
        input: || {
            (
                "from builtins import __name__ as builtin_name\nbuiltin_name\n",
                "builtin_name",
            )
        },
        expect: Expect::Value((Some("builtins.__name__"), "import", 0, 0)),
    },
    Case {
        name: CaseName("builtin_shadow"),
        word: Word::Succeeds,
        input: || ("exec = handler\nexec(source)\n", "exec"),
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("builtin_function_shadow"),
        word: Word::Succeeds,
        input: || ("def f(exec):\n    exec(source)\n", "exec"),
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("builtin_from_import"),
        word: Word::Succeeds,
        input: || {
            (
                "from builtins import exec as execute\nexecute(source)\n",
                "execute",
            )
        },
        expect: Expect::Value((Some("builtins.exec"), "import", 0, 0)),
    },
    Case {
        name: CaseName("builtin_module_import"),
        word: Word::Succeeds,
        input: || ("import builtins\nbuiltins.exec(source)\n", "builtins.exec"),
        expect: Expect::Value((Some("builtins.exec"), "import", 0, 0)),
    },
    Case {
        name: CaseName("post_310_builtin"),
        word: Word::Succeeds,
        input: || ("ExceptionGroup(\"x\", [])\n", "ExceptionGroup"),
        expect: Expect::Value((Some("builtins.ExceptionGroup"), "builtin", 3, 11)),
    },
    Case {
        name: CaseName("post_39_builtin_aiter"),
        word: Word::Succeeds,
        input: || ("aiter(values)\n", "aiter"),
        expect: Expect::Value((Some("builtins.aiter"), "builtin", 3, 10)),
    },
    Case {
        name: CaseName("post_39_builtin_anext"),
        word: Word::Succeeds,
        input: || ("anext(values)\n", "anext"),
        expect: Expect::Value((Some("builtins.anext"), "builtin", 3, 10)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
