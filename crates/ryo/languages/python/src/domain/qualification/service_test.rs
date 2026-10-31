// ignore: idiomatic.arch/file-length - this single lexical-provenance matrix keeps independently authored alias, shadowing, wildcard, version, Unicode and reused-model cases beside one reader; splitting the matrix by arbitrary case counts would hide interactions tested by the common reparse witness
//! Independent Python lexical provenance examples; no reference linter generates expected rows.
use super::{
    constants::BUILTIN_NAMES,
    service::{clear, prepare, qualify},
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    bindings::service::index,
    model::types::Declarations,
    nodes::types::Node,
    scopes::{
        service::{context, declare},
        types::Context,
    },
    syntax::{
        service::{kind, written},
        types::Kind,
    },
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};
type Input = (&'static str, &'static str);
type Value = (Option<&'static str>, &'static str, i64, i64);
type Failure = ();
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
    Case {
        name: CaseName("builtin"),
        word: Word::Succeeds,
        input: || ("exec(source)\n", "exec"),
        expect: Expect::Value((Some("builtins.exec"), "builtin", 3, 0)),
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
fn observe(input: Input, model: &mut Declarations) -> Value {
    clear(model.qualification.get_mut());
    let file = parsed(input.0.as_bytes()).expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    declare(&tree, model);
    index(&tree, model);
    let mut symbols = file.symbols;
    for token in languages::positions(tree.elements).filter(|at| {
        matches!(
            kind(&tree, *at),
            Kind::Token(crate::domain::tokens::types::Token::Name)
        )
    }) {
        assert_eq!(
            languages::text_of(
                &symbols,
                tree.source,
                languages::text_at(tree.elements, token)
            )
            .map(|value| value.0),
            Some(written(&tree, token).0)
        );
    }

    let at = languages::positions(tree.elements)
        .filter(|at| {
            matches!(kind(&tree, *at), Kind::Node(Node::Name | Node::Attribute))
                && context(&tree, *at) == Context::Load
                && written(&tree, *at).0 == input.1
        })
        .last()
        .expect("fixture names a load expression");
    let events = prepare(&tree, model);
    let repeated = prepare(&tree, model);
    assert_eq!(events.hazards, repeated.hazards);
    let answer = qualify(&tree, model, &events, at, &mut symbols);
    (
        languages::text_of(&symbols, tree.source, answer.path)
            .map(|text| &*Box::leak(text.0.to_owned().into_boxed_str())),
        Box::leak(
            languages::text_of(&symbols, tree.source, answer.provenance)
                .expect("known provenance category")
                .0
                .to_owned()
                .into_boxed_str(),
        ),
        answer.since.major.0,
        answer.since.minor.0,
    )
}
fn subject(input: Input) -> Result<Value, ()> {
    assert_eq!(
        BUILTIN_NAMES.iter().map(|name| name.0).collect::<Vec<_>>(),
        include_str!("../../../tests/fixtures/standard/builtin_names.txt")
            .lines()
            .collect::<Vec<_>>()
    );
    let mut model = Declarations::default();
    let first = observe(input, &mut model);
    let _ = observe(("from m import *\nexec(x)\n", "exec"), &mut model);
    clear(model.qualification.get_mut());
    let empty = parsed(b"").expect("empty source");
    let tree = Tree {
        elements: &empty.elements,
        source: ByteSlice(empty.bytes),
        file: languages::FileId(0),
    };
    declare(&tree, &mut model);
    index(&tree, &mut model);
    assert!(prepare(&tree, &model).hazards.is_empty());
    assert_eq!(first, observe(input, &mut model));
    // Numeric target guards: a minor version of 10 must follow 9, not sort lexically before it.
    if input.1 == "aiter" || input.1 == "anext" {
        assert!((first.2, first.3) > (3, 9));
        assert!((first.2, first.3) <= (3, 10));
    }
    if input.1 == "ExceptionGroup" {
        assert!((first.2, first.3) > (3, 10));
        assert!((first.2, first.3) <= (3, 11));
    }
    Ok(first)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
