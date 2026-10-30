//! The `comment` relation of the language: every comment of a file read as json with comments, with its text after the `//` or between
//! the `/*` and the `*/`.

use crate::domain::extras::types::Extra;
use crate::domain::model::types::Model;
use crate::domain::syntax::service::{comment_text, kind_bits};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `comment` relation to, which reads its rows from a file's tree.
pub(crate) struct CommentItem;

ryo::reads! {
    CommentItem => languages::Comment, model(Model) |tree, model, at, symbols| {
        kinds: [Kind::Extra(Extra::LineComment), Kind::Extra(Extra::BlockComment)] as kind_bits,
        row { id: node(at), text: value(comment_text(tree, symbols, at)), file: file, span: span(at), doc: value(languages::Flag::Lowered) }
    }
}
