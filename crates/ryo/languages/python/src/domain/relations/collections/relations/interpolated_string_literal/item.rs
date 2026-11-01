//! Read decoded literal elements from f-strings.

use crate::domain::{
    extras::types::Extra,
    fields::{service::field_of, types::Field},
    model::types::Declarations,
    nodes::types::Node,
    semantic_syntax::service::{positions, visibility},
    syntax::{
        service::{below, kind, parent},
        types::Kind,
    },
    tokens::types::Token,
};
use ryo::languages::{self, Position, Symbols, Tree};

const MISSING: &str = "f-string literal fragment requires its parser-decoded token payload";

/// The optional decoded f-string literal reader.
pub(crate) struct InterpolatedStringLiteralItem;

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

ryo::reads! {
    InterpolatedStringLiteralItem => languages::InterpolatedStringLiteral, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as crate::domain::syntax::service::kind_bits,
        each positions(tree) as element {
            require visibility(tree, model, element, symbols) == crate::domain::semantic_syntax::types::Visibility::Visible;
            require kind(tree, element) == Kind::Extra(Extra::FStringPart);
            when owner(tree, element) {
                None => {}
                Some(owner) => {
                    let mut fragments = below(tree, element).filter(|child| {
                        kind(tree, *child) == Kind::Token(Token::FstringMiddle)
                    });
                    when fragments.find_map(|token| languages::decoded_at(tree.elements, token)) {
                        None => { partial MISSING => languages::Missing::Any; }
                        Some(value) => {
                            given decoded = languages::intern_derived(symbols, tree.source, languages::NameText(value.0));
                            row { id: node(element), owner: node(owner), value: value(decoded), file: file, span: span(element), }
                        }
                    }
                }
            }
        }
    }
}
