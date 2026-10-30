//! The `bindings` service: free functions named by verb that read what a pattern binds, with no IO
//! and no ports. A `let` or a parameter binds a name when its pattern is one name.

use super::constants::NONE;
use super::types::{
    Annotated, Binding, Constness, LetPattern, Patternness, PossibleConstant, Preludeness, Spread,
};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{
    child, end, first_body, first_node, is_root, kind, name_text, node_after, parent, start,
};
use crate::domain::syntax::types::{Kind, Rootness};
use crate::domain::tokens::types::Token;
use ryo::languages;
use ryo::languages::Tree;

/// The `NAME` node a `let` or a parameter at `owner` binds: its pattern is one name, and not a
/// destructuring or a wildcard.
#[must_use]
pub(crate) fn bound_name(
    tree: &Tree<'_>,
    owner: languages::Position,
) -> Option<languages::Position> {
    let pattern = first_body(tree, owner)?;
    if kind(tree, pattern) != Kind::Node(Node::IdentPat) {
        return None;
    }
    child(tree, pattern, Kind::Node(Node::Name))
}

/// Whether a kind of node is a pattern, or a part of one.
fn is_pattern(kind: Kind) -> Patternness {
    if matches!(
        kind,
        Kind::Node(
            Node::BoxPat
                | Node::ConstBlockPat
                | Node::DerefPat
                | Node::IdentPat
                | Node::LiteralPat
                | Node::MacroPat
                | Node::OrPat
                | Node::ParenPat
                | Node::PathPat
                | Node::RangePat
                | Node::RecordPat
                | Node::RecordPatField
                | Node::RecordPatFieldList
                | Node::RefPat
                | Node::RestPat
                | Node::SlicePat
                | Node::TuplePat
                | Node::TupleStructPat
                | Node::WildcardPat
        )
    ) {
        Patternness::Pattern
    } else {
        Patternness::NotPattern
    }
}

/// Whether the name pattern at `at` is nested in the pattern of a `let`, and not the whole pattern: it binds one of the names a
/// destructuring `let` binds.
#[must_use]
pub(crate) fn in_let_pattern(tree: &Tree<'_>, at: languages::Position) -> LetPattern {
    let mut up = at;
    while is_root(tree, up) == Rootness::Inner {
        up = parent(tree, up);
        let here = kind(tree, up);
        if here == Kind::Node(Node::LetStmt) {
            return if first_body(tree, up).is_some_and(|whole| whole != at) {
                LetPattern::In
            } else {
                LetPattern::Outside
            };
        }
        if is_pattern(here) == Patternness::NotPattern {
            return LetPattern::Outside;
        }
    }
    LetPattern::Outside
}

/// Whether the name node at `name` starts as a constant, a unit struct or a variant is written: such a name in a pattern may be one
/// of those and not a binding, which the syntax alone does not say.
#[must_use]
pub(crate) fn looks_like_constant(tree: &Tree<'_>, name: languages::Position) -> Constness {
    let (from, to) = (
        usize::try_from(start(tree, name).0).unwrap_or(usize::MAX),
        usize::try_from(end(tree, name).0).unwrap_or(usize::MAX),
    );
    tree.source
        .0
        .get(from..to)
        .and_then(|bytes| std::str::from_utf8(bytes).ok()) // ignore: idiomatic.arch/no-ok-on-result - the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
        .and_then(|text| text.chars().next())
        .map_or(Constness::NotConstant, |first| {
            if first.is_uppercase() {
                Constness::Constant
            } else {
                Constness::NotConstant
            }
        })
}

/// Whether the name node at `name` is `None`, the one name of the prelude that a pattern writes with no path: it is the variant
/// and not a binding.
#[must_use]
pub(crate) fn is_prelude_variant(tree: &Tree<'_>, name: languages::Position) -> Preludeness {
    let (from, to) = (
        usize::try_from(start(tree, name).0).unwrap_or(usize::MAX),
        usize::try_from(end(tree, name).0).unwrap_or(usize::MAX),
    );
    if tree.source.0.get(from..to) == Some(NONE) {
        Preludeness::Prelude
    } else {
        Preludeness::NotPrelude
    }
}

/// The value a single-name `let` at `at` binds, when it is a string literal.
#[must_use]
pub(crate) fn assigned_string(
    tree: &Tree<'_>,
    at: languages::Position,
) -> Option<languages::Position> {
    node_after(tree, at, Kind::Token(Token::Eq)).filter(|each| {
        kind(tree, *each) == Kind::Node(Node::Literal)
            && child(tree, *each, Kind::Token(Token::String)).is_some()
    })
}

/// Whether the element at `at` is a name in a destructuring `let` that starts as a constant does: it may be one, and then it binds
/// nothing.
#[must_use]
pub(crate) fn names_a_possible_constant(
    tree: &Tree<'_>,
    at: languages::Position,
) -> PossibleConstant {
    if kind(tree, at) == Kind::Node(Node::IdentPat)
        && in_let_pattern(tree, at) == LetPattern::In
        && child(tree, at, Kind::Node(Node::Name)).is_some_and(|name| {
            looks_like_constant(tree, name) == Constness::Constant
                && is_prelude_variant(tree, name) == Preludeness::NotPrelude
        })
    {
        PossibleConstant::Possible
    } else {
        PossibleConstant::Impossible
    }
}

/// The receiver and the parameters of the parameter list at `list`, in order.
pub(crate) fn parameters_of<'a>(
    tree: &'a Tree<'a>,
    list: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    languages::children(tree.elements, list)
        .filter(|each| matches!(kind(tree, *each), Kind::Node(Node::SelfParam | Node::Param)))
}

/// How many arguments the parameter at `parameter` takes: the arguments left over when it is written `...` (the Rust Reference,
/// "External blocks": variadic functions), else one.
#[must_use]
pub(crate) fn spread(tree: &Tree<'_>, parameter: languages::Position) -> Spread {
    if child(tree, parameter, Kind::Token(Token::Dot3)).is_some() {
        Spread::Rest
    } else {
        Spread::One
    }
}

/// What the parameter at `parameter` binds: its name when its pattern is one name, and with it the name's element; otherwise the
/// parameter, which has no name (a destructuring pattern, a wildcard, no pattern at all).
#[must_use]
pub(crate) fn binding_of(tree: &Tree<'_>, parameter: languages::Position) -> Binding {
    match bound_name(tree, parameter)
        .and_then(|name| name_text(tree, name).map(|text| (name, text)))
    {
        Some((id, name)) => Binding { id, name },
        None => Binding {
            id: parameter,
            name: languages::NO_SYMBOL,
        },
    }
}

/// The types the element at `at` annotates, each with what it annotates: the parameters a name or a pattern binds and the result of a
/// function, a closure's parameters, the pattern of a `let`, and a `const` or a `static` item, which its own node is known by.
pub(crate) fn annotations<'a>(
    tree: &'a Tree<'a>,
    at: languages::Position,
) -> impl Iterator<Item = Annotated> + 'a {
    let element = kind(tree, at);
    let function =
        element == Kind::Node(Node::Fn) && child(tree, at, Kind::Node(Node::Name)).is_some();
    let list = if function || element == Kind::Node(Node::ClosureExpr) {
        child(tree, at, Kind::Node(Node::ParamList))
    } else {
        None
    };
    let parameters = list.into_iter().flat_map(move |list| {
        languages::children(tree.elements, list)
            .filter(move |each| kind(tree, *each) == Kind::Node(Node::Param))
            .filter_map(move |parameter| {
                Some(Annotated {
                    owner: bound_name(tree, parameter).unwrap_or(parameter),
                    annotation: node_after(tree, parameter, Kind::Token(Token::Colon))?,
                })
            })
    });
    let returned = if function {
        child(tree, at, Kind::Node(Node::RetType)).and_then(|returned| first_node(tree, returned))
    } else {
        None
    }
    .map(move |annotation| Annotated {
        owner: at,
        annotation,
    });
    let declared = if element == Kind::Node(Node::LetStmt) {
        bound_name(tree, at)
            .or_else(|| first_body(tree, at))
            .zip(node_after(tree, at, Kind::Token(Token::Colon)))
    } else {
        None
    }
    .map(|(owner, annotation)| Annotated { owner, annotation });
    let item = if matches!(element, Kind::Node(Node::Const | Node::Static)) {
        node_after(tree, at, Kind::Token(Token::Colon))
    } else {
        None
    }
    .map(|annotation| Annotated {
        owner: at,
        annotation,
    });
    parameters.chain(returned).chain(declared).chain(item)
}
