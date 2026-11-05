//! The `scopes` service's cases: the scope an element is in.

use super::service::{enclosing, scopeness};
use super::types::Scopeness;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind;
use crate::domain::syntax::types::Kind;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

/// A source.
type Input = &'static str;
/// The kind of the scope each call in it is in, in order.
type Value = &'static [Kind];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_call_in_a_function_is_in_the_function"),
        word: Word::Succeeds,
        input: || "func f() { g() }\n",
        expect: Expect::Value(&[Kind::Node(Node::FuncDecl)]),
    },
    Case {
        name: CaseName("a_call_in_a_function_literal_is_in_the_literal"),
        word: Word::Succeeds,
        input: || "func f() { g := func() { h() }; g() }\n",
        expect: Expect::Value(&[Kind::Node(Node::FuncLit), Kind::Node(Node::FuncDecl)]),
    },
    Case {
        name: CaseName("a_call_outside_any_function_is_in_the_file"),
        word: Word::Succeeds,
        input: || "var x = f()\n",
        expect: Expect::Value(&[Kind::Node(Node::File)]),
    },
];

fn subject(source: Input) -> Result<Value, Failure> {
    if scopeness(Kind::Node(Node::FuncDecl)) != Scopeness::Scope
        || scopeness(Kind::Node(Node::Ident)) != Scopeness::Inner
    {
        return Err(());
    }
    let mut file = parsed(source.as_bytes()).map_err(drop)?;
    let elements = std::mem::take(&mut file.elements);
    let tree = Tree {
        elements: &elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let scopes: Vec<Kind> = languages::positions(&elements)
        .filter(|at| kind(&tree, *at) == Kind::Node(Node::CallExpr))
        .map(|at| kind(&tree, enclosing(&tree, at)))
        .collect();
    Ok(&*Box::leak(scopes.into_boxed_slice()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
