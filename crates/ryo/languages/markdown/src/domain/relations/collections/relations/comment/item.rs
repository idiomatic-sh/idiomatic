//! The `comment` relation of the language: every HTML comment, a block of its own or in a line of text, with its text between
//! `<!--` and `-->`.

use crate::domain::model::types::Sections;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{comment_text, commentness, kind_bits};
use crate::domain::syntax::types::{Commentness, Kind};
use ryo::languages;

/// The reader the `parse` contract hands the `comment` relation to, which reads its rows from a file's tree.
pub(crate) struct CommentItem;

ryo::reads! {
    CommentItem => languages::Comment, model(Sections) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Html)] as kind_bits,
        require (commentness(tree, at) == Commentness::Comment);
        row { id: node(at), text: value(comment_text(tree, symbols, at)), file: file, span: span(at), doc: value(languages::Flag::Lowered) }
    }
}
