//! The `expressions` service: free functions named by verb over the library's tree, with no IO and no ports. It finds the nodes of
//! `go/ast` the expressions and types the library groups otherwise are made of.

use ryo::languages::Count;

use super::types::Conversion;
use crate::domain::declarations::service::variadic;
use crate::domain::grammar::service::{
    ending, field, first, form, named, named_count, parent_form, spelled, start, stop,
};
use crate::domain::grammar::types::{Concrete, Form, Index, Role};
use crate::domain::nodes::types::Node;
use crate::domain::pieces::service::node;
use crate::domain::pieces::types::Pieces;
use crate::domain::tokens::types::Token;

/// The `Index` node `go/ast` makes of an operand and the `count` type arguments after it.
const fn instance(count: Count) -> Node {
    if count.0 > 1 {
        Node::IndexListExpr
    } else {
        Node::IndexExpr
    }
}

/// How many type arguments the node `arguments` writes.
fn arguments_in(tree: &Concrete<'_>, arguments: Index) -> Count {
    Count(named(tree, arguments).count())
}

/// Finds the nodes of `A | B` or `~A`: a `Field` in an interface, and a chain of `BinaryExpr` when it has several terms.
pub(crate) fn union(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    let (from, to) = (start(tree, at), ending(tree, at));
    if parent_form(tree, at) == Some(Form::Interface) {
        node(pieces, Node::Field, from, to);
    }
    for member in named(tree, at).skip(1) {
        node(pieces, Node::BinaryExpr, from, ending(tree, member));
    }
}

/// Finds the index `go/ast` makes of a generic type instantiated: one type argument is an `IndexExpr`, several an `IndexListExpr`.
pub(crate) fn generic_type(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    let count = field(tree, at, Role::TypeArguments)
        .map_or(Count(1), |arguments| arguments_in(tree, arguments));
    node(pieces, instance(count), start(tree, at), ending(tree, at));
}

/// Finds the index `go/ast` makes of a generic function instantiated: an operand and its type arguments.
pub(crate) fn instantiation(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    let count = Count(named_count(tree, at).0.saturating_sub(1));
    node(pieces, instance(count), start(tree, at), ending(tree, at));
}

/// Finds the nodes of a call: the call, the instantiation of a generic function in it, and, for a conversion to a pointer type, the
/// `*` of the conversion: `*(*T)(x)` is read by the library as the conversion to the pointer type `*(*T)`, and by `go/parser` as `*`
/// of the conversion `(*T)(x)`.
pub(crate) fn call(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index, shape: Conversion) {
    let (from, to) = (start(tree, at), ending(tree, at));
    let mut called = from;
    if shape == Conversion::Written {
        let mut kind = field(tree, at, Role::Type);
        while let Some(pointer) =
            kind.filter(|each| form(tree, *each) == Form::Node(Node::StarExpr))
        {
            node(pieces, Node::StarExpr, start(tree, pointer), to);
            pieces.consumed.push(pointer);
            kind = named(tree, pointer).last();
            called = kind.map_or(called, |each| start(tree, each));
        }
    }
    node(pieces, Node::CallExpr, called, to);
    if let Some(arguments) = field(tree, at, Role::TypeArguments) {
        node(
            pieces,
            instance(arguments_in(tree, arguments)),
            from,
            ending(tree, arguments),
        );
    }
}

/// Finds the node of `*x` or `-x`: a `StarExpr` for a dereference, a `UnaryExpr` for any other operator.
pub(crate) fn unary(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    let star = first(tree, at).is_some_and(|each| form(tree, each) == Form::Spelled(Token::Mul));
    let kind = if star {
        Node::StarExpr
    } else {
        Node::UnaryExpr
    };
    node(pieces, kind, start(tree, at), ending(tree, at));
}

/// Finds the nodes of a type with braces, `interface { ... }`: an `InterfaceType` and the `FieldList` that holds its elements.
pub(crate) fn interface(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    let to = ending(tree, at);
    node(pieces, Node::InterfaceType, start(tree, at), to);
    if let Some(open) = spelled(tree, at, Token::Lbrace) {
        node(pieces, Node::FieldList, start(tree, open), to);
    }
}

/// Finds the nodes of a field of a struct: a `Field`, and a `StarExpr` over the type of an embedded pointer.
pub(crate) fn field_declaration(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    let to = ending(tree, at);
    node(pieces, Node::Field, start(tree, at), to);
    if let (Some(star), Some(kind)) = (
        first(tree, at).filter(|each| form(tree, *each) == Form::Spelled(Token::Mul)),
        field(tree, at, Role::Type),
    ) {
        node(
            pieces,
            Node::StarExpr,
            start(tree, star),
            ending(tree, kind),
        );
    }
}

/// Finds the nodes of a parameter that takes the arguments left over: a `Field` that holds an `Ellipsis`.
pub(crate) fn variadic_parameter(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    node(pieces, Node::Field, start(tree, at), ending(tree, at));
    variadic(tree, pieces, at);
}

/// Finds the nodes of `[...]T`: an `ArrayType`, whose length is an `Ellipsis`.
pub(crate) fn implicit_array(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    node(pieces, Node::ArrayType, start(tree, at), ending(tree, at));
    if let Some(dots) = spelled(tree, at, Token::Ellipsis) {
        node(pieces, Node::Ellipsis, start(tree, dots), stop(tree, dots));
    }
}

/// Finds the node of the braces of a composite literal: none when the literal's type is before them, a `CompositeLit` when they stand
/// alone, as the elements of an outer literal do.
pub(crate) fn elements(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    if parent_form(tree, at) != Some(Form::Node(Node::CompositeLit)) {
        node(
            pieces,
            Node::CompositeLit,
            start(tree, at),
            ending(tree, at),
        );
    }
}
