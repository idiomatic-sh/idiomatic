//! Lexical cases derived from Python's execution model: binding operations, resolution of names,
//! class scope boundaries, and comprehension scope. These expectations do not consult a linter.
//! Source: <https://docs.python.org/3/reference/executionmodel.html#resolution-of-names>.

use super::service::{index, resolve};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::Declarations;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::{context, declare};
use crate::domain::scopes::types::Context;
use crate::domain::syntax::service::{kind, name_of};
use crate::domain::syntax::types::Kind;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

/// Source and the name whose last read is resolved.
type Input = (&'static str, &'static str);
/// The line of the lexical declaration, or no file-local binding.
type Value = Option<usize>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_class_body_can_read_its_import"),
        word: Word::Succeeds,
        input: || ("class C:\n    import os\n    x = os\n", "os"),
        expect: Expect::Value(Some(2)),
    },
    Case {
        name: CaseName("a_function_name_is_not_bound_in_its_default"),
        word: Word::Succeeds,
        input: || ("def f(x=f): pass\n", "f"),
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("a_class_name_is_not_bound_in_its_body"),
        word: Word::Succeeds,
        input: || ("class C:\n    x = C\n", "C"),
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("a_function_can_refer_to_itself_in_its_body"),
        word: Word::Succeeds,
        input: || ("def f():\n    return f()\n", "f"),
        expect: Expect::Value(Some(1)),
    },
    Case {
        name: CaseName("an_import_binds_the_base_of_an_attribute"),
        word: Word::Succeeds,
        input: || ("import os\nos.path.exists('x')\n", "os"),
        expect: Expect::Value(Some(1)),
    },
    Case {
        name: CaseName("parameters_shadow_module_imports"),
        word: Word::Succeeds,
        input: || ("import os\ndef f(os):\n    return os\n", "os"),
        expect: Expect::Value(Some(2)),
    },
    Case {
        name: CaseName("local_binding_precedes_initialization_lexically"),
        word: Word::Succeeds,
        input: || ("import os\ndef f():\n    print(os)\n    os = 1\n", "os"),
        expect: Expect::Value(Some(4)),
    },
    Case {
        name: CaseName("a_closure_can_read_a_later_module_binding"),
        word: Word::Succeeds,
        input: || ("def f():\n    return os\nimport os\n", "os"),
        expect: Expect::Value(Some(3)),
    },
    Case {
        name: CaseName("a_class_body_can_read_its_earlier_binding"),
        word: Word::Succeeds,
        input: || ("class C:\n    x = 1\n    y = x\n", "x"),
        expect: Expect::Value(Some(2)),
    },
    Case {
        name: CaseName("methods_skip_the_class_namespace"),
        word: Word::Succeeds,
        input: || {
            (
                "x = 1\nclass C:\n    x = 2\n    def f(self):\n        return x\n",
                "x",
            )
        },
        expect: Expect::Value(Some(1)),
    },
    Case {
        name: CaseName("class_comprehensions_skip_the_class_namespace"),
        word: Word::Succeeds,
        input: || {
            (
                "x = 1\nclass C:\n    x = 2\n    y = [x for i in range(3)]\n",
                "x",
            )
        },
        expect: Expect::Value(Some(1)),
    },
    Case {
        name: CaseName("comprehension_targets_are_local"),
        word: Word::Succeeds,
        input: || ("x = 1\ny = [x for x in range(3)]\n", "x"),
        expect: Expect::Value(Some(2)),
    },
    Case {
        name: CaseName("comprehension_targets_do_not_escape"),
        word: Word::Succeeds,
        input: || ("y = [x for x in range(3)]\nprint(x)\n", "x"),
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("global_redirects_skip_a_closure_binding"),
        word: Word::Succeeds,
        input: || {
            (
                "x = 1\ndef f():\n    x = 2\n    def g():\n        global x\n        return x\n",
                "x",
            )
        },
        expect: Expect::Value(Some(1)),
    },
    Case {
        name: CaseName("a_missing_global_does_not_fall_back_to_a_closure"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    x = 2\n    def g():\n        global x\n        return x\n",
                "x",
            )
        },
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("nonlocal_reads_find_an_enclosing_function"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    x = 2\n    def g():\n        nonlocal x\n        return x\n",
                "x",
            )
        },
        expect: Expect::Value(Some(2)),
    },
    Case {
        name: CaseName("module_assignment_reads_the_previous_binding"),
        word: Word::Succeeds,
        input: || ("import x\nx = x\n", "x"),
        expect: Expect::Value(Some(1)),
    },
    Case {
        name: CaseName("a_star_import_does_not_invent_a_local_declaration"),
        word: Word::Succeeds,
        input: || ("from module import *\nprint(x)\n", "x"),
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("explicit_reexports_retain_the_import_binding"),
        word: Word::Succeeds,
        input: || ("from module import x as x\nprint(x)\n", "x"),
        expect: Expect::Value(Some(1)),
    },
    Case {
        name: CaseName("all_strings_do_not_become_lexical_reads"),
        word: Word::Succeeds,
        input: || ("from module import x\n__all__ = ['x']\n", "x"),
        expect: Expect::Value(None),
    },
];

fn subject((source, wanted): Input) -> Value {
    let file = parsed(source.as_bytes()).expect("the language case parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    declare(&tree, &mut model);
    index(&tree, &mut model);
    let expected = model.bindings.clone();
    // Rebuilding a retained file model must replace its index, never append to it.
    index(&tree, &mut model);
    assert_eq!(model.bindings, expected);
    let empty = parsed(b"").expect("empty Python parses");
    let empty_tree = Tree {
        elements: &empty.elements,
        source: ByteSlice(empty.bytes),
        file: languages::FileId(0),
    };
    declare(&empty_tree, &mut model);
    index(&empty_tree, &mut model);
    assert!(model.bindings.is_empty());
    declare(&tree, &mut model);
    index(&tree, &mut model);
    assert_eq!(model.bindings, expected);
    let occurrence = languages::positions(&file.elements)
        .filter(|at| {
            kind(&tree, *at) == Kind::Node(Node::Name)
                && context(&tree, *at) == Context::Load
                && name_of(&tree, *at)
                    .and_then(|name| languages::text_of(&file.symbols, tree.source, name))
                    .is_some_and(|name| name.0 == wanted)
        })
        .last()?;
    let binding = resolve(&tree, &model, occurrence, name_of(&tree, occurrence)?)?;
    let Ok(offset) = usize::try_from(languages::start_at(tree.elements, binding).0) else {
        return None;
    };
    Some(
        source
            .get(..offset)?
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1,
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
