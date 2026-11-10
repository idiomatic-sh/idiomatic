//! Read decoded literal elements from f-strings.

use super::constants::MISSING;
use crate::domain::{
    extras::types::Extra,
    fields::service::field_of,
    labels::types::Field,
    model::types::Declarations,
    nodes::types::Node,
    semantic_syntax::service::{positions, visibility},
    syntax::{
        service::{below, kind, parent},
        types::Kind,
    },
    tokens::types::Token,
};
use ryo::languages::{self, NameText, Position, Tree};

/// The optional decoded f-string literal reader.
pub(crate) struct InterpolatedStringLiteralItem;

/// Find the enclosing f-string while excluding format-specification owners.
fn owner(tree: &Tree<'_>, at: Position) -> Option<Position> {
    let mut child = at;
    let mut ancestor = parent(tree, child);
    while child != ancestor {
        if kind(tree, ancestor) == Kind::Node(Node::JoinedStr) {
            return (field_of(tree, ancestor) != Field::FormatSpec).then_some(ancestor);
        }
        child = ancestor;
        ancestor = parent(tree, ancestor);
    }
    None
}

/// Identify the token carrying one literal element’s decoded payload.
fn fragment_token(tree: &Tree<'_>, at: Position) -> Option<Token> {
    match (kind(tree, at), kind(tree, parent(tree, at))) {
        (Kind::Node(Node::Constant), Kind::Extra(Extra::FStringPart)) => Some(Token::FstringMiddle),
        (Kind::Extra(Extra::StringPart), Kind::Node(Node::JoinedStr)) => Some(Token::String),
        _ => None,
    }
}

/// Borrow a literal element’s decoded token text, or report its absence.
fn decoded_fragment<'a>(tree: &'a Tree<'_>, at: Position) -> Result<NameText<'a>, ()> {
    let token_kind = fragment_token(tree, at).ok_or(())?;
    let token = below(tree, at)
        .find(|child| kind(tree, *child) == Kind::Token(token_kind))
        .ok_or(())?;
    languages::decoded_at(tree.elements, token).ok_or(())
}

ryo::reads! {
    InterpolatedStringLiteralItem => languages::InterpolatedStringLiteral, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as crate::domain::syntax::service::kind_bits,
        each (positions(tree)) as element {
            require visibility(tree, model, element, symbols) == crate::domain::semantic_syntax::types::Visibility::Visible;
            require fragment_token(tree, element).is_some();
            when owner(tree, element) {
                None => {}
                Some(owner) => {
                    when decoded_fragment(tree, element) {
                        Err(()) => { partial MISSING => languages::Missing::Any; }
                        Ok(decoded) => {
                            given value = languages::intern_derived(symbols, tree.source, decoded);
                            row { id: node(element), owner: node(owner), value: value(value), file: file, span: span(element), }
                        }
                    }
                }
            }
        }
    }
}
