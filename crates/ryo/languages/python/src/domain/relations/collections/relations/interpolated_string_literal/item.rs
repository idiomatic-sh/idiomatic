//! Read decoded literal elements from f-strings.

use super::constants::MISSING;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::literal_facts::service::fragments;
use crate::domain::node::service::{node_cell, nodes_of_kind, span_at, word};
use crate::domain::node::types::Numbered;
use crate::domain::nodes::types::Node as TypedNode;
use crate::domain::syntax::types::Kind as TypedKind;
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

// The engine fills `interpolated-string-literal` from the walked file: every row of the file, once.
ryo::fills! {
    InterpolatedStringLiteralItem => languages::InterpolatedStringLiteral, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::InterpolatedStringLiteral as languages::CatalogRelation>::Row;
        let mut pieces = Vec::new();
        for Numbered { id, node } in nodes_of_kind(&file.syntax, TypedKind::Node(TypedNode::JoinedStr)) {
            pieces.clear();
            fragments(node, &mut pieces);
            for piece in &pieces {
                rows.write(Row {
                    id: node_cell(&file.syntax, piece.id),
                    owner: node_cell(&file.syntax, id),
                    value: word(words, piece.value),
                    file: file.syntax.file,
                    span: span_at(&file.syntax, piece.id),
                });
            }
        }
    }
}
