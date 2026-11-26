//! The `mention` relation of the language: an identifier written directly in a token tree, where the syntax does not say whether it
//! is a use. A macro the language reads as expressions has its uses in `reference` too.

use crate::domain::model::types::ReadIndexes;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::{kind, kind_bits, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::domain::file_view::macros::fills_rows;
use ryo::languages;

/// The reader the `fills_rows!` below binds the `mention` relation's rows to.
pub(crate) struct MentionItem;

// The engine fills the relation from the walked file, by the same reader.
fills_rows! {
    MentionItem => languages::Mention, model(ReadIndexes) |tree, _model, at, _symbols| {
        kinds: [Kind::Token(Token::Ident)] as kind_bits,
        require kind(tree, parent(tree, at)) == Kind::Node(Node::TokenTree);
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            target: text(at),
            file: file,
            span: span(at),
        }
    }
}
