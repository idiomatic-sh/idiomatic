//! The `prologues` service: free functions named by verb over the syntax tree, with no IO and no ports. A directive is a string
//! statement in a prologue: at the start of a file, or of a function body, with only string statements before it.

use super::constants::NO_ARGUMENT;
use crate::domain::names::service::{nodes, text};
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::service::{kind, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Position, Symbol, Symbols, Tree};

/// The string literal that is the whole of the statement at `at`, when the statement is a string statement.
fn literal_of(tree: &Tree<'_>, at: Position) -> Option<Position> {
    if kind(tree, at) != Kind::Node(TypeScriptNode::ExpressionStatement) {
        return None;
    }
    let first = languages::children(tree.elements, at).next()?;
    if kind(tree, first) != Kind::Node(TypeScriptNode::Literal) {
        return None;
    }
    languages::children(tree.elements, first)
        .any(|child| kind(tree, child) == Kind::Token(Token::String))
        .then_some(first)
}

/// The text of a string literal between its quotes, as written: an escape is not decoded, so `"use\x20strict"` is not `use strict`.
fn written(tree: &Tree<'_>, symbols: &mut Symbols, literal: Position) -> Option<Symbol> {
    let token = languages::children(tree.elements, literal)
        .find(|child| kind(tree, *child) == Kind::Token(Token::String))?;
    let raw = text(tree, token).0;
    let inner = raw.get(1..raw.len().checked_sub(1)?)?;
    Some(languages::intern_derived(
        symbols,
        tree.source,
        languages::NameText(inner),
    ))
}

/// The directive the statement at `at` is, as written between its quotes, when it is in a prologue.
pub(crate) fn directive(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Option<Symbol> {
    let literal = literal_of(tree, at)?;
    let enclosing = parent(tree, at);
    let holds_prologue = match kind(tree, enclosing) {
        Kind::Node(TypeScriptNode::Program) => true,
        Kind::Node(TypeScriptNode::BlockStatement) => matches!(
            kind(tree, parent(tree, enclosing)),
            Kind::Node(
                TypeScriptNode::FunctionDeclaration
                    | TypeScriptNode::FunctionExpression
                    | TypeScriptNode::ArrowFunctionExpression
            )
        ),
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => false,
    };
    if !holds_prologue {
        return None;
    }
    // A directive is only one when every statement before it is a string statement too.
    if !nodes(tree, enclosing)
        .take_while(|statement| *statement != at)
        .all(|statement| literal_of(tree, statement).is_some())
    {
        return None;
    }
    written(tree, symbols, literal)
}

/// The text of a directive: empty, because a prologue directive has no argument.
pub(crate) fn argument(tree: &Tree<'_>, symbols: &mut Symbols) -> Symbol {
    languages::intern_derived(symbols, tree.source, languages::NameText(NO_ARGUMENT))
}
