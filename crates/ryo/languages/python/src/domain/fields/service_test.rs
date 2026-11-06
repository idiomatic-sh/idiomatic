//! The `fields` service's cases: the role of an edge of the tree under its parent, one row for each role of the vocabulary, on source
//! read through the parse. The roles are the fields of `Python.asdl`; the adapter's cases hold them to what `CPython`'s `ast.dump`
//! prints for every pinned file.

use super::service::role_of;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind;
use crate::domain::syntax::service::parent;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::inbound::contracts::parse::contract::ParseContract;
use crate::outbound::notes::factory::postcard;
use crate::outbound::syntax::factory::ruff_python_parser;
use ryo::languages::Position;
use ryo::languages::Tree;
use std::sync::Arc;

/// An edge of the tree of some source: the nth edge, in source order, from an element of a kind to an element of a kind.
#[derive(Clone, Copy)]
struct Edge {
    source: &'static str,
    parent: Kind,
    child: Kind,
    nth: usize,
}

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

const fn edge(source: &'static str, parent: Kind, child: Kind, nth: usize) -> Edge {
    Edge {
        source,
        parent,
        child,
        nth,
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("none_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "pass\n",
                Kind::Node(Node::Pass),
                Kind::Token(Token::Name),
                0,
            )
        },
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("body_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "pass\n",
                Kind::Node(Node::Module),
                Kind::Node(Node::Pass),
                0,
            )
        },
        expect: Expect::Value("body"),
    },
    Case {
        name: CaseName("name_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f(): pass\n",
                Kind::Node(Node::FunctionDef),
                Kind::Extra(Extra::Identifier),
                0,
            )
        },
        expect: Expect::Value("name"),
    },
    Case {
        name: CaseName("args_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f(x)\n",
                Kind::Extra(Extra::CallArguments),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("args"),
    },
    Case {
        name: CaseName("decorator_list_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "@d\ndef f(): pass\n",
                Kind::Node(Node::FunctionDef),
                Kind::Extra(Extra::Decorator),
                0,
            )
        },
        expect: Expect::Value("decorator-list"),
    },
    Case {
        name: CaseName("returns_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f() -> int: pass\n",
                Kind::Node(Node::FunctionDef),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("returns"),
    },
    Case {
        name: CaseName("type_params_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "def f[T](): pass\n",
                Kind::Node(Node::FunctionDef),
                Kind::Extra(Extra::TypeParams),
                0,
            )
        },
        expect: Expect::Value("type-params"),
    },
    Case {
        name: CaseName("bases_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "class C(B): pass\n",
                Kind::Extra(Extra::CallArguments),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("bases"),
    },
    Case {
        name: CaseName("keywords_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "f(k=1)\n",
                Kind::Extra(Extra::CallArguments),
                Kind::Node(Node::Keyword),
                0,
            )
        },
        expect: Expect::Value("keywords"),
    },
    Case {
        name: CaseName("value_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "x = 1\n",
                Kind::Node(Node::Assign),
                Kind::Node(Node::Constant),
                0,
            )
        },
        expect: Expect::Value("value"),
    },
    Case {
        name: CaseName("targets_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "x = 1\n",
                Kind::Node(Node::Assign),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("targets"),
    },
    Case {
        name: CaseName("target_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "x += 1\n",
                Kind::Node(Node::AugAssign),
                Kind::Node(Node::Name),
                0,
            )
        },
        expect: Expect::Value("target"),
    },
    Case {
        name: CaseName("op_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "a + b\n",
                Kind::Node(Node::BinOp),
                Kind::Token(Token::Plus),
                0,
            )
        },
        expect: Expect::Value("op"),
    },
    Case {
        name: CaseName("annotation_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "x: int\n",
                Kind::Node(Node::AnnAssign),
                Kind::Node(Node::Name),
                1,
            )
        },
        expect: Expect::Value("annotation"),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    let language = ParseContract::new(
        crate::settings::PythonParserSettings::default(),
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
    );
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    languages::contract::build(
        &language,
        languages::ParseRequest {
            path: languages::SourcePath("item.py"),
            file: languages::FileId(0),
            bytes: languages::ByteSlice(given.source.as_bytes()),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
    )
    .map_err(drop)?;
    let tree = Tree {
        elements: &elements,
        source: languages::ByteSlice(given.source.as_bytes()),
        file: languages::FileId(0),
    };
    let found = languages::positions(&elements)
        .filter(|at| *at != Position(0))
        .filter(|at| {
            kind(&tree, parent(&tree, *at)) == given.parent && kind(&tree, *at) == given.child
        })
        .nth(given.nth)
        .ok_or(())?;
    assert_eq!(role_of(&tree, found), languages::field_at(&elements, found));
    let name = languages::text_of(
        &symbols,
        tree.source,
        languages::field_name_at(&elements, found),
    )
    .map_or("none", |name| {
        &*Box::leak(name.0.to_owned().into_boxed_str())
    });
    Ok(name)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod control_flow_roles_test;
#[cfg(test)]
mod coverage_test;
#[cfg(test)]
mod expression_roles_test;
#[cfg(test)]
mod pattern_and_parameter_roles_test;
