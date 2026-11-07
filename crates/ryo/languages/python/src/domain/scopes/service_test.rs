//! The `scopes` service's cases: the scope a name is evaluated or bound in.

use super::service::{binding_scope, context, declare, enclosing};
use super::types::Context;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::Declarations;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, name_of};
use crate::domain::syntax::types::Kind;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

/// A source and the name whose first name expression is asked about.
type Input = (&'static str, &'static str);
/// The kind of the scope it is evaluated in, the kind of the scope that binds it, and what it does with the name.
type Value = (Kind, Kind, Context);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_in_a_function_is_the_function_s"),
        word: Word::Succeeds,
        input: || ("def f():\n    x = 1\n", "x"),
        expect: Expect::Value((
            Kind::Node(Node::FunctionDef),
            Kind::Node(Node::FunctionDef),
            Context::Store,
        )),
    },
    Case {
        name: CaseName("a_global_name_is_bound_in_the_module"),
        word: Word::Succeeds,
        input: || ("def f():\n    global x\n    x = 1\n", "x"),
        expect: Expect::Value((
            Kind::Node(Node::FunctionDef),
            Kind::Node(Node::Module),
            Context::Store,
        )),
    },
    Case {
        name: CaseName("a_nonlocal_name_is_bound_in_the_enclosing_function"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    def g():\n        nonlocal x\n        x = 1\n",
                "x",
            )
        },
        expect: Expect::Value((
            Kind::Node(Node::FunctionDef),
            Kind::Node(Node::FunctionDef),
            Context::Store,
        )),
    },
    Case {
        name: CaseName("an_assignment_expression_in_a_comprehension_binds_outside_it"),
        word: Word::Succeeds,
        input: || ("[(y := i) for i in r]\n", "y"),
        expect: Expect::Value((
            Kind::Node(Node::ListComp),
            Kind::Node(Node::Module),
            Context::Store,
        )),
    },
    Case {
        name: CaseName("the_first_iterable_of_a_comprehension_is_evaluated_outside_it"),
        word: Word::Succeeds,
        input: || ("[i for i in r]\n", "r"),
        expect: Expect::Value((
            Kind::Node(Node::Module),
            Kind::Node(Node::Module),
            Context::Load,
        )),
    },
    Case {
        name: CaseName("a_default_is_evaluated_outside_its_function"),
        word: Word::Succeeds,
        input: || ("def f(a=d): pass\n", "d"),
        expect: Expect::Value((
            Kind::Node(Node::Module),
            Kind::Node(Node::Module),
            Context::Load,
        )),
    },
    Case {
        name: CaseName("del_deletes"),
        word: Word::Succeeds,
        input: || ("del x\n", "x"),
        expect: Expect::Value((
            Kind::Node(Node::Module),
            Kind::Node(Node::Module),
            Context::Del,
        )),
    },
];

fn subject((source, wanted): Input) -> Result<Value, Failure> {
    let file = parsed(source.as_bytes()).expect("the row's source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    declare(&tree, &mut model);
    let at = languages::positions(&file.elements)
        .find(|each| {
            kind(&tree, *each) == Kind::Node(Node::Name)
                && name_of(&tree, *each)
                    .and_then(|name| languages::text_of(&file.symbols, tree.source, name))
                    .map(|text| text.0)
                    == Some(wanted)
        })
        .ok_or(())?;
    let name = name_of(&tree, at).ok_or(())?;
    Ok((
        kind(&tree, enclosing(&tree, at)),
        kind(&tree, binding_scope(&tree, &model, at, name)),
        context(&tree, at),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
