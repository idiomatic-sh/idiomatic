//! The `comment` relation of the language: every comment's content without its delimiters, and whether rustdoc reads it.

use crate::domain::model::types::ReadIndexes;
use crate::domain::syntax::service::{comment_text, documentation, kind_bits};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `comment` relation's rows to.
pub(crate) struct CommentItem;

ryo::reads! {
    CommentItem => languages::Comment, model(ReadIndexes) |tree, _model, at, symbols| {
        kinds: [Kind::Token(Token::Comment)] as kind_bits,
        row { id: node(at), text: value(comment_text(tree, symbols, at)), file: file, span: span(at), doc: value(documentation(tree, at)) }
    }
}
