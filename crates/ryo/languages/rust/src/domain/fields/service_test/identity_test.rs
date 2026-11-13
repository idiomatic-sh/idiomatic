//! The `fields` service's cases for identity: the role of an edge of the tree under its parent.

use super::super::fixtures::{Edge, edge};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What a row is given.
type Input = Edge;
/// The name of the role the edge plays.
type Value = &'static str;
type Failure = ();

/// Reads one row's input through the concept fixtures' shared reading.
fn subject(input: Input) -> Result<Value, Failure> {
    super::super::fixtures::subject(input)
}

/// The rows of this part's cases.
pub(super) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("none_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() {}",
                Kind::Node(Node::Fn),
                Kind::Token(Token::FnKw),
                0,
            )
        },
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("name_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || edge("fn f() {}", Kind::Node(Node::Fn), Kind::Node(Node::Name), 0),
        expect: Expect::Value("name"),
    },
    Case {
        name: CaseName("name_ref_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { g(); }",
                Kind::Node(Node::PathSegment),
                Kind::Node(Node::NameRef),
                0,
            )
        },
        expect: Expect::Value("name-ref"),
    },
    Case {
        name: CaseName("path_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { g(); }",
                Kind::Node(Node::PathExpr),
                Kind::Node(Node::Path),
                0,
            )
        },
        expect: Expect::Value("path"),
    },
    Case {
        name: CaseName("segment_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a::b(); }",
                Kind::Node(Node::Path),
                Kind::Node(Node::PathSegment),
                0,
            )
        },
        expect: Expect::Value("segment"),
    },
    Case {
        name: CaseName("qualifier_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { a::b(); }",
                Kind::Node(Node::Path),
                Kind::Node(Node::Path),
                0,
            )
        },
        expect: Expect::Value("qualifier"),
    },
    Case {
        name: CaseName("visibility_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "pub fn f() {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::Visibility),
                0,
            )
        },
        expect: Expect::Value("visibility"),
    },
    Case {
        name: CaseName("attrs_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "#[a] fn f() {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::Attr),
                0,
            )
        },
        expect: Expect::Value("attrs"),
    },
    Case {
        name: CaseName("lifetime_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f<'a>() {}",
                Kind::Node(Node::LifetimeParam),
                Kind::Node(Node::Lifetime),
                0,
            )
        },
        expect: Expect::Value("lifetime"),
    },
    Case {
        name: CaseName("label_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "fn f() { 'a: loop {} }",
                Kind::Node(Node::LoopExpr),
                Kind::Node(Node::Label),
                0,
            )
        },
        expect: Expect::Value("label"),
    },
    Case {
        name: CaseName("abi_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "extern \"C\" fn f() {}",
                Kind::Node(Node::Fn),
                Kind::Node(Node::Abi),
                0,
            )
        },
        expect: Expect::Value("abi"),
    },
    Case {
        name: CaseName("rename_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "use a as b;",
                Kind::Node(Node::UseTree),
                Kind::Node(Node::Rename),
                0,
            )
        },
        expect: Expect::Value("rename"),
    },
    Case {
        name: CaseName("self_ty_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "impl A for B {}",
                Kind::Node(Node::Impl),
                Kind::Node(Node::PathType),
                1,
            )
        },
        expect: Expect::Value("self-ty"),
    },
    Case {
        name: CaseName("trait_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "impl A for B {}",
                Kind::Node(Node::Impl),
                Kind::Node(Node::PathType),
                0,
            )
        },
        expect: Expect::Value("trait"),
    },
    Case {
        name: CaseName("keyword_is_the_role_of_its_edge"),
        word: Word::Succeeds,
        input: || {
            edge(
                "#[cfg(unix)] fn f() {}",
                Kind::Node(Node::CfgAtom),
                Kind::Token(Token::Ident),
                0,
            )
        },
        expect: Expect::Value("keyword"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
