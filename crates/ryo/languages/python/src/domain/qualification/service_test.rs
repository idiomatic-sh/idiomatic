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
#[derive(Clone, Copy, Debug)]
struct Input {
    source: &'static str,
    expression: &'static str,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shown {
    path: Option<&'static str>,
    provenance: &'static str,
    major: i64,
    minor: i64,
}
type Value = Shown;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("module_import"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\ncontextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("aliased_module"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib as c\nc.suppress()\n",
            expression: "c.suppress",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("from_import"),
        word: Word::Succeeds,
        input: || Input {
            source: "from contextlib import suppress\nsuppress()\n",
            expression: "suppress",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("aliased_from_import"),
        word: Word::Succeeds,
        input: || Input {
            source: "from contextlib import suppress as s\ns()\n",
            expression: "s",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("unaliased_dotted_import"),
        word: Word::Succeeds,
        input: || Input {
            source: "import xml.etree\nxml.etree.parse()\n",
            expression: "xml.etree.parse",
        },
        expect: Expect::Value(Shown {
            path: Some("xml.etree.parse"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("aliased_dotted_import"),
        word: Word::Succeeds,
        input: || Input {
            source: "import xml.etree as e\ne.parse()\n",
            expression: "e.parse",
        },
        expect: Expect::Value(Shown {
            path: Some("xml.etree.parse"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("relative_import_not_absolute"),
        word: Word::Succeeds,
        input: || Input {
            source: "from .contextlib import suppress\nsuppress()\n",
            expression: "suppress",
        },
        expect: Expect::Value(Shown {
            path: Some(".contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("before_import_immediate"),
        word: Word::Succeeds,
        input: || Input {
            source: "contextlib.suppress()\nimport contextlib\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "unresolved",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("before_import_deferred"),
        word: Word::Succeeds,
        input: || Input {
            source: "def f():\n    contextlib.suppress()\nimport contextlib\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("rebound_module"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\ncontextlib = object()\ncontextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "local",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("later_rebinding_deferred"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\ndef f():\n    contextlib.suppress()\ncontextlib = object()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "local",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("function_parameter"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\ndef f(contextlib):\n    contextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "local",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("future_function_local"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\ndef f():\n    contextlib.suppress()\n    contextlib = object()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "local",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("class_body"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\nclass C:\n    contextlib = object()\n    contextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "local",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("method_skips_class"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\nclass C:\n    contextlib = object()\n    def f(self):\n        contextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("class_default_uses_class"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\nclass C:\n    contextlib = object()\n    def f(self, x=contextlib.suppress()): pass\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "local",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("deleted_import"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\ndel contextlib\ncontextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "ambiguous",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("reimport_after_delete"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\ndel contextlib\nimport contextlib\ncontextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("wildcard_ambiguity"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\nfrom m import *\ncontextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "ambiguous",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("explicit_after_wildcard"),
        word: Word::Succeeds,
        input: || Input {
            source: "from m import *\nimport contextlib\ncontextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("builtin"),
        word: Word::Succeeds,
        input: || Input {
            source: "exec(source)\n",
            expression: "exec",
        },
        expect: Expect::Value(Shown {
            path: Some("builtins.exec"),
            provenance: "builtin",
            major: 3,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("builtin_shadow"),
        word: Word::Succeeds,
        input: || Input {
            source: "exec = handler\nexec(source)\n",
            expression: "exec",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "local",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("builtin_function_shadow"),
        word: Word::Succeeds,
        input: || Input {
            source: "def f(exec):\n    exec(source)\n",
            expression: "exec",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "local",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("builtin_from_import"),
        word: Word::Succeeds,
        input: || Input {
            source: "from builtins import exec as execute\nexecute(source)\n",
            expression: "execute",
        },
        expect: Expect::Value(Shown {
            path: Some("builtins.exec"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("builtin_module_import"),
        word: Word::Succeeds,
        input: || Input {
            source: "import builtins\nbuiltins.exec(source)\n",
            expression: "builtins.exec",
        },
        expect: Expect::Value(Shown {
            path: Some("builtins.exec"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("post_310_builtin"),
        word: Word::Succeeds,
        input: || Input {
            source: "ExceptionGroup(\"x\", [])\n",
            expression: "ExceptionGroup",
        },
        expect: Expect::Value(Shown {
            path: Some("builtins.ExceptionGroup"),
            provenance: "builtin",
            major: 3,
            minor: 11,
        }),
    },
    Case {
        name: CaseName("post_39_builtin_aiter"),
        word: Word::Succeeds,
        input: || Input {
            source: "aiter(values)\n",
            expression: "aiter",
        },
        expect: Expect::Value(Shown {
            path: Some("builtins.aiter"),
            provenance: "builtin",
            major: 3,
            minor: 10,
        }),
    },
    Case {
        name: CaseName("post_39_builtin_anext"),
        word: Word::Succeeds,
        input: || Input {
            source: "anext(values)\n",
            expression: "anext",
        },
        expect: Expect::Value(Shown {
            path: Some("builtins.anext"),
            provenance: "builtin",
            major: 3,
            minor: 10,
        }),
    },
    Case {
        name: CaseName("ordinary_dynamic"),
        word: Word::Succeeds,
        input: || Input {
            source: "def f(x):\n    x()\n",
            expression: "x",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "local",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("mixed_dynamic_and_known"),
        word: Word::Succeeds,
        input: || Input {
            source: "def f(x):\n    x()\nimport contextlib\ncontextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("receiver_call_is_dynamic"),
        word: Word::Succeeds,
        input: || Input {
            source: "factory().suppress()\n",
            expression: "factory().suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "unresolved",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("global_binding"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\ndef f():\n    global contextlib\n    contextlib.suppress()\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("comprehension_local"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\n[contextlib.suppress() for contextlib in values]\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: None,
            provenance: "local",
            major: 0,
            minor: 0,
        }),
    },
    Case {
        name: CaseName("comprehension_outer_iterable"),
        word: Word::Succeeds,
        input: || Input {
            source: "import contextlib\n[x for contextlib in contextlib.suppress()]\n",
            expression: "contextlib.suppress",
        },
        expect: Expect::Value(Shown {
            path: Some("contextlib.suppress"),
            provenance: "import",
            major: 0,
            minor: 0,
        }),
    },
];
fn observe(input: Input, model: &mut Declarations) -> Value {
    clear(model.qualification.get_mut());
    let file = parsed(input.source.as_bytes()).expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    declare(&tree, model);
    index(&tree, model);
    let mut symbols = file.symbols;
    let at = languages::positions(tree.elements)
        .filter(|at| {
            matches!(kind(&tree, *at), Kind::Node(Node::Name | Node::Attribute))
                && context(&tree, *at) == Context::Load
                && written(&tree, *at).0 == input.expression
        })
        .last()
        .expect("fixture names a load expression");
    let events = prepare(&tree, model);
    let repeated = prepare(&tree, model);
    assert_eq!(events.hazards, repeated.hazards);
    let answer = qualify(&tree, model, &events, at, &mut symbols);
    Shown {
        path: languages::text_of(&symbols, tree.source, answer.path)
            .map(|text| &*Box::leak(text.0.to_owned().into_boxed_str())),
        provenance: Box::leak(
            languages::text_of(&symbols, tree.source, answer.provenance)
                .expect("known provenance category")
                .0
                .to_owned()
                .into_boxed_str(),
        ),
        major: answer.since.major.0,
        minor: answer.since.minor.0,
    }
}
fn subject(input: Input) -> Result<Value, Failure> {
    assert_eq!(
        BUILTIN_NAMES.iter().map(|name| name.0).collect::<Vec<_>>(),
        include_str!("../../../tests/fixtures/standard/builtin_names.txt")
            .lines()
            .collect::<Vec<_>>()
    );
    let mut model = Declarations::default();
    let first = observe(input, &mut model);
    let _ = observe(
        Input {
            source: "from m import *\nexec(x)\n",
            expression: "exec",
        },
        &mut model,
    );
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
    if input.expression == "aiter" || input.expression == "anext" {
        assert!((first.major, first.minor) > (3, 9));
        assert!((first.major, first.minor) <= (3, 10));
    }
    if input.expression == "ExceptionGroup" {
        assert!((first.major, first.minor) > (3, 10));
        assert!((first.major, first.minor) <= (3, 11));
    }
    Ok(first)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
