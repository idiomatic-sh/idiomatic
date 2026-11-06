//! Independent Python lexical provenance examples; every case also checks the shared retained-model lifecycle.
use super::service::builtin_named;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use ryo::languages::NameText;
type Input = (&'static str, &'static str);
type Value = (Option<&'static str>, &'static str, i64, i64);
type Failure = String;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("wildcard_preserves_builtin"),
        word: Word::Succeeds,
        input: || ("from m import *\nexec(code)\n", "exec"),
        expect: Expect::Value((Some("builtins.exec"), "builtin", 3, 0)),
    },
    Case {
        name: CaseName("wildcard_unresolved_name"),
        word: Word::Succeeds,
        input: || ("from m import *\nmissing_name()\n", "missing_name"),
        expect: Expect::Value((None, "ambiguous", 0, 0)),
    },
    Case {
        name: CaseName("wildcard_preserves_explicit_local"),
        word: Word::Succeeds,
        input: || ("exec = custom\nfrom m import *\nexec(code)\n", "exec"),
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("assignment_after_wildcard_shadows_builtin"),
        word: Word::Succeeds,
        input: || ("from m import *\nexec = custom\nexec(code)\n", "exec"),
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("wildcard_does_not_restore_deleted_import"),
        word: Word::Succeeds,
        input: || {
            (
                "import contextlib\ndel contextlib\nfrom m import *\ncontextlib.suppress()\n",
                "contextlib.suppress",
            )
        },
        expect: Expect::Value((None, "ambiguous", 0, 0)),
    },
    Case {
        name: CaseName("nfkc_builtin"),
        word: Word::Succeeds,
        input: || ("𝗲𝘅𝗲𝗰(code)\n", "𝗲𝘅𝗲𝗰"),
        expect: Expect::Value((Some("builtins.exec"), "builtin", 3, 0)),
    },
    Case {
        name: CaseName("nfkc_import_alias"),
        word: Word::Succeeds,
        input: || ("import logging as 𝗹𝗼𝗴\nlog.Logger()\n", "log.Logger"),
        expect: Expect::Value((Some("logging.Logger"), "import", 0, 0)),
    },
    Case {
        name: CaseName("nfkc_import_module"),
        word: Word::Succeeds,
        input: || ("import 𝗹𝗼𝗴𝗴𝗶𝗻𝗴\nlogging.Logger()\n", "logging.Logger"),
        expect: Expect::Value((Some("logging.Logger"), "import", 0, 0)),
    },
    Case {
        name: CaseName("nfkc_import_attribute"),
        word: Word::Succeeds,
        input: || ("import logging\nlogging.𝗟𝗼𝗴𝗴𝗲𝗿()\n", "logging.𝗟𝗼𝗴𝗴𝗲𝗿"),
        expect: Expect::Value((Some("logging.Logger"), "import", 0, 0)),
    },
    Case {
        name: CaseName("nfkc_builtin_shadow"),
        word: Word::Succeeds,
        input: || ("𝗲𝘅𝗲𝗰 = custom\nexec(code)\n", "exec"),
        expect: Expect::Value((None, "local", 0, 0)),
    },
    Case {
        name: CaseName("nfkc_dotted_import"),
        word: Word::Succeeds,
        input: || ("import 𝘅𝗺𝗹.etree\nxml.etree.parse()\n", "xml.etree.parse"),
        expect: Expect::Value((Some("xml.etree.parse"), "import", 0, 0)),
    },
    Case {
        name: CaseName("nfkc_from_import"),
        word: Word::Succeeds,
        input: || {
            (
                "from builtins import 𝗲𝘅𝗲𝗰 as execute\nexecute(code)\n",
                "execute",
            )
        },
        expect: Expect::Value((Some("builtins.exec"), "import", 0, 0)),
    },
    Case {
        name: CaseName("nfkc_parameter_shadow"),
        word: Word::Succeeds,
        input: || ("def f(𝗲𝘅𝗲𝗰):\n    exec(code)\n", "exec"),
        expect: Expect::Value((None, "local", 0, 0)),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    super::fixtures::replay(input).map_err(|error| error.clone())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(CHECKS, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}

fn the_builtin_catalog_answers_for_builtins_only() {
    assert!(builtin_named(NameText("exec")).is_some());
    assert!(builtin_named(NameText("not_a_builtin_name")).is_none());
}

/// Each check below states its own source and asserts its own outcome.
const CHECKS: &[Case<fn(), (), ()>] = &[Case {
    name: CaseName("the_builtin_catalog_answers_for_builtins_only"),
    word: Word::Succeeds,
    input: || the_builtin_catalog_answers_for_builtins_only,
    expect: Expect::Value(()),
}];

#[cfg(test)]
mod builtin_names_test;
#[cfg(test)]
mod dynamic_names_test;
#[cfg(test)]
mod import_forms_test;
