//! The `comment` relation of the language: every comment, with its text after the `#`. None is documentation: Python documents with
//! a docstring, a string the language reads as `__doc__`, which is a `string_literal` row and no comment.

use crate::domain::model::types::Declarations;
use crate::domain::syntax::service::{comment_text, kind_bits};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// Reads the `comment` rows of one Python file from its syntax tree.
pub(crate) struct CommentItem;

ryo::reads! {
    CommentItem => languages::Comment, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Token(Token::Comment)] as kind_bits,
        row { id: node(at), text: value(comment_text(tree, symbols, at)), file: file, span: span(at), doc: value(languages::Flag::Lowered) }
    }
}
