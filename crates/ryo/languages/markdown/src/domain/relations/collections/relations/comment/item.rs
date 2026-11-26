//! The `comment` relation of the language: every HTML comment, a block of its own or in a line of text, with its text between
//! `<!--` and `-->`.

use crate::domain::file_view::service::{node_cell, span_at, written, word};
use crate::domain::file_view::types::MarkdownWalk;
use crate::domain::syntax::constants::{COMMENT_CLOSE, COMMENT_OPEN};
use ryo::languages::node_table;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct CommentItem;

// The engine fills `comment` from the walked file: every raw HTML construct that is a comment.
ryo::fills! {
    CommentItem => languages::Comment, walk(MarkdownWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Comment as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Node(Node::Html))) {
            let text = written(&syntax, id).trim_start();
            if !text.starts_with(COMMENT_OPEN) {
                continue;
            }
            let inner = text.get(COMMENT_OPEN.len()..).unwrap_or_default();
            let inner = inner.split(COMMENT_CLOSE).next().unwrap_or_default();
            rows.write(Row {
                id: node_cell(&syntax, id),
                text: word(words, inner),
                file: syntax.file,
                span: span_at(&syntax, id),
                doc: languages::Flag::Lowered,
            });
        }
    }
}
