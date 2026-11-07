//! The import forms cases of the parent test file.

use super::{Failure, Input, Value, subject};
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("module_import"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\ncontextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("aliased_module"),
        word: Word::Succeeds,
        input: || ("import contextlib as c\nc.suppress()\n", "c.suppress"),
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("from_import"),
        word: Word::Succeeds,
        input: || ("from contextlib import suppress\nsuppress()\n", "suppress"),
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("aliased_from_import"),
        word: Word::Succeeds,
        input: || ("from contextlib import suppress as s\ns()\n", "s"),
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("unaliased_dotted_import"),
        word: Word::Succeeds,
        input: || ("import xml.etree\nxml.etree.parse()\n", "xml.etree.parse"),
        expect: Expect::Value((Some("xml.etree.parse"), "import", 0, 0)),
    },
    Case {
        name: CaseName("aliased_dotted_import"),
        word: Word::Succeeds,
        input: || ("import xml.etree as e\ne.parse()\n", "e.parse"),
        expect: Expect::Value((Some("xml.etree.parse"), "import", 0, 0)),
    },
    Case {
        name: CaseName("relative_import_not_absolute"),
        word: Word::Succeeds,
        input: || ("from .contextlib import suppress\nsuppress()\n", "suppress"),
        expect: Expect::Value((Some(".contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("before_import_immediate"),
        word: Word::Succeeds,
        input: || {
            (
                "contextlib.suppress()\nimport contextlib\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((None, "unresolved", 0, 0)),
    },
    Case {
        name: CaseName("before_import_deferred"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    contextlib.suppress()\nimport contextlib\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("rebound_module"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\ncontextlib = object()\ncontextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("later_rebinding_deferred"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\ndef f():\n    contextlib.suppress()\ncontextlib = object()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("function_parameter"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\ndef f(contextlib):\n    contextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("future_function_local"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\ndef f():\n    contextlib.suppress()\n    contextlib = object()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("class_body"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\nclass C:\n    contextlib = object()\n    contextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("method_skips_class"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\nclass C:\n    contextlib = object()\n    def f(self):\n        contextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("class_default_uses_class"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\nclass C:\n    contextlib = object()\n    def f(self, x=contextlib.suppress()): pass\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("deleted_import"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\ndel contextlib\ncontextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((None, "ambiguous", 0, 0)),
    },
    Case {
        name: CaseName("reimport_after_delete"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\ndel contextlib\nimport contextlib\ncontextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("wildcard_preserves_known_import"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\nfrom m import *\ncontextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((Some("contextlib.suppress"), "import", 0, 0)),
    },
    Case {
        name: CaseName("explicit_after_wildcard"),
        word: Word::Succeeds,
        input: || {
            (
                "from m import *\nimport contextlib\ncontextlib.suppress()\n",
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
