//! The `signatures` service: free functions named by verb over the domain's types, with no IO and no ports. A function's receiver,
//! name, type parameters, parameters and results are under its `FuncType`, as `go/ast` has it; a field of a list of parentheses holds
//! the names it declares and then their type: all its nodes but the last are names, unless it has only one, which is a type.

use ryo::languages::{self, Count, Integer, ParameterKind, Position, Tree};

use super::constants::{PARAMETER, RECEIVER, RESULT, VARIADIC};
use super::types::{Callable, Listing, Parameter, Signature};
use crate::domain::names::service::unknown;
use crate::domain::names::types::Unresolved;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, kind, nodes, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// Whether the list at `list` is written between brackets: the type parameters of a function or a type.
fn bracketed(tree: &Tree<'_>, list: Position) -> Option<Position> {
    languages::children(tree.elements, list)
        .next()
        .filter(|first| kind(tree, *first) == Kind::Token(Token::Lbrack))
}

/// The parts of the function type at `function_type`.
fn of_type(tree: &Tree<'_>, function_type: Position) -> Signature {
    let named = nodes(tree, function_type).any(|each| kind(tree, each) == Kind::Node(Node::Ident));
    let mut found = Signature::default();
    let mut passed = false;
    for each in nodes(tree, function_type) {
        if kind(tree, each) == Kind::Node(Node::Ident) {
            found.name = Some(each);
            passed = true;
            continue;
        }
        if kind(tree, each) == Kind::Node(Node::FieldList) {
            if named && !passed {
                found.receiver = Some(each);
            } else if bracketed(tree, each).is_some() {
                found.type_parameters = Some(each);
            } else if found.parameters.is_none() {
                found.parameters = Some(each);
            } else {
                found.results = Some(each);
            }
        }
    }
    found
}

/// The parts of the function declaration or literal at `function`, and of the method of an interface, a `Field`.
#[must_use]
pub(crate) fn signature(tree: &Tree<'_>, function: Position) -> Signature {
    let Some(function_type) =
        nodes(tree, function).find(|each| kind(tree, *each) == Kind::Node(Node::FuncType))
    else {
        return Signature::default();
    };
    let mut found = of_type(tree, function_type);
    found.body = child(tree, function, Kind::Node(Node::BlockStmt));
    found
}

/// What the list of fields at `list` is.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a list of fields is told by what holds it; every other holder has no list of fields"
)]
pub(crate) fn listing(tree: &Tree<'_>, list: Position) -> Option<Listing> {
    let holder = parent(tree, list);
    match kind(tree, holder) {
        Kind::Node(Node::StructType) => Some(Listing::Members),
        Kind::Node(Node::InterfaceType) => Some(Listing::Methods),
        Kind::Node(Node::TypeSpec) => Some(Listing::TypeParameters),
        Kind::Node(Node::FuncType) => {
            let parts = of_type(tree, holder);
            [
                (parts.receiver, Listing::Receiver),
                (parts.type_parameters, Listing::TypeParameters),
                (parts.parameters, Listing::Parameters),
                (parts.results, Listing::Results),
            ]
            .into_iter()
            .find(|(place, _)| *place == Some(list))
            .map(|(_, listing)| listing)
        }
        _ => None,
    }
}

/// The function a list of parameters, results or a receiver at `list` belongs to: a declaration, a literal, or the method of an
/// interface; none for the list of a function type that is no function.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only a declaration, a literal and the method of an interface are called"
)]
pub(crate) fn callable(tree: &Tree<'_>, list: Position) -> Option<Callable> {
    let function_type = parent(tree, list);
    if kind(tree, function_type) != Kind::Node(Node::FuncType) {
        return None;
    }
    let holder = parent(tree, function_type);
    match kind(tree, holder) {
        Kind::Node(Node::FuncDecl | Node::FuncLit) => Some(Callable { callable: holder }),
        Kind::Node(Node::Field)
            if listing(tree, parent(tree, holder)) == Some(Listing::Methods) =>
        {
            Some(Callable { callable: holder })
        }
        _ => None,
    }
}

/// How many nodes a field holds: all its names, and then their type.
fn held(tree: &Tree<'_>, field: Position) -> Count {
    Count(nodes(tree, field).count())
}

/// How many names a field declares: all its nodes but the last, unless it has only one, which is a type.
fn named(tree: &Tree<'_>, field: Position) -> Count {
    Count(held(tree, field).0.saturating_sub(1))
}

/// How many places a field takes in its list: one for each name, one for a field with none.
fn width(tree: &Tree<'_>, field: Position) -> Count {
    Count(named(tree, field).0.max(1))
}

/// The place of the parameter `offset` places after the first of a field that starts `before` places into its list.
fn place(before: Count, offset: Count) -> Integer {
    Integer(i64::try_from(before.0 + offset.0).unwrap_or(i64::MAX))
}

/// The parameters, the receiver or the named results the field at `field` declares, in order; none for a field that is none of them.
/// A field that declares no name declares one parameter with none, unless it is a result.
pub(crate) fn parameters<'a>(
    tree: &'a Tree<'_>,
    field: Position,
) -> impl Iterator<Item = Parameter> + use<'a> {
    let list = parent(tree, field);
    let listed = listing(tree, list).zip(callable(tree, list));
    let declared = named(tree, field);
    let rows = match listed.map(|(listing, _)| listing) {
        Some(Listing::Results) => declared,
        Some(Listing::Receiver | Listing::Parameters) if declared.0 > 0 => declared,
        Some(Listing::Receiver | Listing::Parameters) => Count(held(tree, field).0.min(1)),
        Some(Listing::Members | Listing::Methods | Listing::TypeParameters) | None => Count(0),
    };
    let before = nodes(tree, list)
        .take_while(|each| *each != field)
        .fold(Count(0), |total, each| Count(total.0 + width(tree, each).0));
    let variadic = child(tree, field, Kind::Node(Node::Ellipsis)).is_some()
        || nodes(tree, field).any(|each| kind(tree, each) == Kind::Node(Node::Ellipsis));
    (0..rows.0).filter_map(move |offset| {
        let (listing, callable) = listed?;
        let (kind, word) = match listing {
            Listing::Receiver => (ParameterKind::Receiver, RECEIVER),
            Listing::Results => (ParameterKind::Result, RESULT),
            Listing::Parameters if variadic => (ParameterKind::VariadicPositional, VARIADIC),
            Listing::Parameters => (ParameterKind::Positional, PARAMETER),
            Listing::Members | Listing::Methods | Listing::TypeParameters => return None,
        };
        let (element, name) = if declared.0 == 0 {
            (nodes(tree, field).next()?, unknown(Unresolved::Unnamed))
        } else {
            let element = nodes(tree, field).nth(offset)?;
            (
                element,
                languages::first_token_text(tree.elements, element)?,
            )
        };
        Some(Parameter {
            kind,
            word,
            named: element,
            name,
            ordinal: place(before, Count(offset)),
            callable: callable.callable,
        })
    })
}
