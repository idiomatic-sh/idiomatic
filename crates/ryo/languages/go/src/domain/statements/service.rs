//! The `statements` service: free functions named by verb over the library's tree, with no IO and no ports. It finds the nodes of
//! `go/ast` the statements that hold clauses, loops, receives, labels and jumps are made of.

use crate::domain::grammar::service::raw;
use crate::domain::grammar::service::{
    child_count, children, empty_label, ending, field, form, spelled, start, stop,
};
use crate::domain::grammar::types::{Concrete, Form, Index, Naming, Role};
use crate::domain::nodes::types::Node;
use crate::domain::pieces::service::{node, token};
use crate::domain::pieces::types::Pieces;
use crate::domain::tokens::types::Token;

/// Finds a statement that holds clauses, `kind`, and the `BlockStmt` that holds them.
pub(crate) fn clauses(tree: &Concrete<'_>, pieces: &mut Pieces, kind: Node, at: Index) {
    let to = ending(tree, at);
    node(pieces, kind, start(tree, at), to);
    if let Some(open) = spelled(tree, at, Token::Lbrace) {
        node(pieces, Node::BlockStmt, start(tree, open), to);
    }
}

/// Finds the statement `go/ast` makes of the header of a type switch, `x := y.(type)` or `y.(type)`, and the assertion in it.
pub(crate) fn type_switch(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    clauses(tree, pieces, Node::TypeSwitchStmt, at);
    let Some(value) = field(tree, at, Role::Value) else {
        return;
    };
    let close = children(tree, at)
        .skip_while(|each| start(tree, *each) < stop(tree, value))
        .find(|each| {
            raw(tree, *each).naming == Naming::Anonymous
                && form(tree, *each) == Form::Spelled(Token::Rparen)
        })
        .map_or_else(|| ending(tree, value), |each| stop(tree, each));
    let alias = field(tree, at, Role::Alias);
    let from = alias.map_or_else(|| start(tree, value), |each| start(tree, each));
    let header = if alias.is_some() {
        Node::AssignStmt
    } else {
        Node::ExprStmt
    };
    node(pieces, header, from, close);
    node(pieces, Node::TypeAssertExpr, start(tree, value), close);
}

/// Finds a `for`: a `RangeStmt` when it has a range clause, a `ForStmt` otherwise.
pub(crate) fn repetition(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    let ranges = children(tree, at).any(|each| form(tree, each) == Form::RangeClause);
    let kind = if ranges {
        Node::RangeStmt
    } else {
        Node::ForStmt
    };
    node(pieces, kind, start(tree, at), ending(tree, at));
}

/// Finds a receive of a `select`: an `AssignStmt` when it assigns, an `ExprStmt` when it does not.
pub(crate) fn receive(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    let kind = if field(tree, at, Role::Left).is_some() {
        Node::AssignStmt
    } else {
        Node::ExprStmt
    };
    node(pieces, kind, start(tree, at), ending(tree, at));
}

/// Finds a labeled statement, and the empty statement `go/parser` makes after a label that has none.
pub(crate) fn labeled(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index) {
    let to = ending(tree, at);
    node(pieces, Node::LabeledStmt, start(tree, at), to);
    if empty_label(tree, at).is_some() {
        node(pieces, Node::EmptyStmt, to, to);
    }
}

/// Finds a statement the library names after the keyword it starts with, `jump`: a `BranchStmt`, and the keyword when nothing is under it.
pub(crate) fn branch(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index, jump: Token) {
    let (from, to) = (start(tree, at), ending(tree, at));
    node(pieces, Node::BranchStmt, from, to);
    if child_count(tree, at).0 == 0 {
        token(pieces, jump, from, to);
    }
}
