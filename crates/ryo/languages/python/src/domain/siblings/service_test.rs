//! The `siblings` service's cases: where an element of a parsed source stands among its siblings, and the role that alone decides for
//! a parameter, a statement of a block, a token under its node and a node under an extra.

use super::service::{block, index, parameter, placement, preceding, token_under, within};
use super::types::Placement;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::extras::types::Extra;
use crate::domain::labels::{service::field_name, types::Field};
use crate::domain::nodes::types::Node;
use crate::domain::semantic_syntax::service::positions;
use crate::domain::syntax::{
    service::{kind, kind_name, parent, written},
    types::Kind,
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Position, Tree};

/// What a row asks of a source.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The role of every parameter, each element right under the `arguments` it stands in.
    Parameters(&'static str),
    /// The block role of every expression statement.
    Blocks(&'static str),
    /// The role of every token under a node, where it has one.
    Tokens(&'static str),
    /// The role of every node under an extra, where it has one.
    Within(&'static str),
    /// Every name's place among the nodes of its parent, whether one follows, and the token before it.
    Order(&'static str),
}
/// What a row is given.
type Input = Given;
/// Each element as `text:answer`, joined by `|`.
type Value = &'static str;
/// A source the parser refuses.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("parameters_take_their_section_from_the_slash_and_the_star"),
        word: Word::Succeeds,
        input: || Given::Parameters("def f(a, /, b, *c, d, **e): pass\n"),
        expect: Expect::Value("a:posonlyargs|b:args|*c:vararg|d:kwonlyargs|**e:kwarg"),
    },
    Case {
        name: CaseName("statements_after_else_and_finally_take_their_blocks"),
        word: Word::Succeeds,
        input: || Given::Blocks("try:\n    a\nexcept E:\n    b\nelse:\n    c\nfinally:\n    d\n"),
        expect: Expect::Value("a:body|b:body|c:orelse|d:finalbody"),
    },
    Case {
        name: CaseName("operators_literals_and_identifiers_take_their_token_roles"),
        word: Word::Succeeds,
        input: || Given::Tokens("x += -a + b < 2\nfrom .. import m\nf'{v!r}'\n"),
        expect: Expect::Value(
            "x:id|+=:op|-:op|a:id|+:op|b:id|<:ops|2:value|.:level|.:level|v:id|r:conversion",
        ),
    },
    Case {
        name: CaseName("call_arguments_bases_and_defaults_take_their_roles_under_an_extra"),
        word: Word::Succeeds,
        input: || Given::Within("f(a, k=b)\nclass C(B): pass\ndef g(p=1, *, q=2): pass\n"),
        expect: Expect::Value(
            "a:args|k=b:keywords|B:bases|p:args|1:defaults|q:kwonlyargs|2:kw-defaults",
        ),
    },
    Case {
        name: CaseName("names_have_their_place_among_their_siblings"),
        word: Word::Succeeds,
        input: || Given::Order("f(a, -b, c)\n"),
        expect: Expect::Value("f:0followed|a:0followed|b:0lastminus|c:2lastcomma"),
    },
];

/// The text of an element and its answer.
fn shown(tree: &Tree<'_>, at: Position, answer: &str) -> String {
    format!("{}:{answer}", written(tree, at).0)
}

/// Answers the row's question for every element it names.
fn subject(given: Input) -> Result<Value, Failure> {
    let (Given::Parameters(source)
    | Given::Blocks(source)
    | Given::Tokens(source)
    | Given::Within(source)
    | Given::Order(source)) = given;
    let file = parsed(source.as_bytes()).map_err(|_| ())?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let texts: Vec<String> = positions(&tree)
        .filter(|at| *at != Position(0))
        .filter_map(|at| {
            let element = kind(&tree, at);
            let holder = kind(&tree, parent(&tree, at));
            let role = |field: Field| {
                (field != Field::None).then(|| shown(&tree, at, field_name(field).0))
            };
            match (given, element, holder) {
                (
                    Given::Parameters(_),
                    Kind::Node(_) | Kind::Extra(Extra::ParameterWithDefault),
                    Kind::Node(Node::Arguments),
                ) => role(parameter(&tree, at)),
                (Given::Blocks(_), Kind::Node(Node::Expr), _) => role(block(&tree, at)),
                (Given::Tokens(_), Kind::Token(_), Kind::Node(node)) => {
                    role(token_under(&tree, at, node))
                }
                (Given::Within(_), _, Kind::Extra(extra)) => role(within(&tree, at, extra)),
                (Given::Order(_), Kind::Node(Node::Name), _) => Some(shown(
                    &tree,
                    at,
                    &format!(
                        "{}{}{}",
                        index(&tree, at).0,
                        match placement(&tree, at) {
                            Placement::Last => "last",
                            Placement::Followed => "followed",
                        },
                        preceding(&tree, at).map_or("", |before| kind_name(before).0)
                    ),
                )),
                _ => None,
            }
        })
        .collect();
    Ok(Box::leak(texts.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
