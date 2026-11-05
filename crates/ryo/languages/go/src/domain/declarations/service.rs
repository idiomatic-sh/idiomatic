//! The `declarations` service: free functions named by verb over the library's tree, with no IO and no ports. It finds the nodes of
//! `go/ast` a function, a function type, a list of parameters and a declaration are made of.

use super::types::Grouping;
use crate::domain::grammar::service::{children, ending, field, form, parent_form, spelled, start};
use crate::domain::grammar::types::{ByteIndex, Concrete, Form, Index, Role};
use crate::domain::nodes::types::GoNode;
use crate::domain::pieces::service::{node, over};
use crate::domain::pieces::types::Pieces;
use crate::domain::tokens::types::Token;

/// Where the function type of the signature at `at` ends: after its results, or its parameters.
fn signature_end(tree: &Concrete<'_>, at: Index) -> ByteIndex {
    field(tree, at, Role::Result)
        .or_else(|| field(tree, at, Role::Parameters))
        .map_or_else(|| ending(tree, at), |each| ending(tree, each))
}

/// Finds the results of a function type, which `go/ast` holds in a `FieldList` and a `Field` even when they are one type without
/// parentheses.
fn results(tree: &Concrete<'_>, pieces: &mut Pieces, signature: Index) {
    if let Some(result) = field(tree, signature, Role::Result)
        && form(tree, result) != Form::Parameters
    {
        over(pieces, tree, GoNode::FieldList, result);
        over(pieces, tree, GoNode::Field, result);
    }
}

/// Finds the nodes of a function declaration or a method: a `FuncDecl` and its `FuncType`, besides the nodes of its children.
pub(crate) fn function(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    node(pieces, GoNode::FuncDecl, start(tree, at), ending(tree, at));
    node(
        pieces,
        GoNode::FuncType,
        start(tree, at),
        signature_end(tree, at),
    );
    results(tree, pieces, at);
}

/// Finds the nodes of a function literal: a `FuncLit` and its `FuncType`, besides the nodes of its children.
pub(crate) fn closure(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    node(pieces, GoNode::FuncLit, start(tree, at), ending(tree, at));
    node(
        pieces,
        GoNode::FuncType,
        start(tree, at),
        signature_end(tree, at),
    );
    results(tree, pieces, at);
}

/// Finds the nodes of a function type: a `FuncType` and its results, besides the nodes of its children.
pub(crate) fn signature(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    node(pieces, GoNode::FuncType, start(tree, at), ending(tree, at));
    results(tree, pieces, at);
}

/// Finds the nodes of a method of an interface: a `Field` that holds a `FuncType` starting at the parameters.
pub(crate) fn method_element(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    node(pieces, GoNode::Field, start(tree, at), ending(tree, at));
    let parameters =
        field(tree, at, Role::Parameters).map_or(start(tree, at), |each| start(tree, each));
    node(pieces, GoNode::FuncType, parameters, signature_end(tree, at));
    results(tree, pieces, at);
}

/// Finds the `Ellipsis` of a variadic parameter, from its dots to the end of its type.
pub(crate) fn variadic(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    if let (Some(dots), Some(kind)) = (
        spelled(tree, at, Token::Ellipsis),
        field(tree, at, Role::Type),
    ) {
        node(
            pieces,
            GoNode::Ellipsis,
            start(tree, dots),
            ending(tree, kind),
        );
    }
}

/// Whether the declaration at `at` is a parameter the library writes as a type alone, in front of the names that share the type of the
/// last one: it names nothing itself, and only a type that is a name can be one of those names.
fn bare(tree: &Concrete<'_>, at: Index, last_named: ByteIndex) -> Option<ByteIndex> {
    let type_is_name =
        field(tree, at, Role::Type).map(|each| form(tree, each)) == Some(Form::TypeName);
    (form(tree, at) == Form::Node(GoNode::Field)
        && start(tree, at) < last_named
        && field(tree, at, Role::Name).is_none()
        && type_is_name)
        .then(|| start(tree, at))
}

/// Finds the fields of the list of parameters at `list`. The library may read `a, b, c, d, e, f, g, h, i, j int` as a list of types
/// followed by a field of the last names and the type (its parser keeps a limited number of readings of the list); a list that names
/// any of its parameters names all, so a type alone in front of a named field is one of its names, and the field starts at the first of
/// them. A list that names none has a field for each declaration, which the walk finds as it meets them.
pub(crate) fn fields(tree: &Concrete<'_>, pieces: &mut Pieces, list: Index) -> Grouping {
    let last_named = children(tree, list)
        .filter(|each| {
            matches!(form(tree, *each), Form::Node(GoNode::Field) | Form::Variadic)
                && field(tree, *each, Role::Name).is_some()
        })
        .last()
        .map(|each| start(tree, each));
    let Some(last_named) = last_named else {
        return Grouping::Ungrouped;
    };
    let mut group: Option<ByteIndex> = None;
    for child in children(tree, list) {
        if let Some(from) = bare(tree, child, last_named) {
            group = group.or(Some(from));
            continue;
        }
        if matches!(form(tree, child), Form::Node(GoNode::Field) | Form::Variadic) {
            let from = group.take().unwrap_or_else(|| start(tree, child));
            node(pieces, GoNode::Field, from, ending(tree, child));
            if form(tree, child) == Form::Variadic {
                variadic(tree, pieces, child);
            }
        }
    }
    Grouping::Grouped
}

/// Whether the declaration at `at` is a statement of a block: the statement `go/ast` holds a declaration in.
fn is_statement(tree: &Concrete<'_>, at: Index) -> Option<Form> {
    parent_form(tree, at).filter(|each| matches!(each, Form::StatementList | Form::Labeled))
}

/// Finds the nodes of a `const`, `var`, `type` or `import` declaration: a `GenDecl`, in a `DeclStmt` where a statement is.
pub(crate) fn declaration(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    let (from, to) = (start(tree, at), ending(tree, at));
    if form(tree, at) == Form::Declaration && is_statement(tree, at).is_some() {
        node(pieces, GoNode::DeclStmt, from, to);
    }
    node(pieces, GoNode::GenDecl, from, to);
}
