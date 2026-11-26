//! The `comment` relation of the language: every comment, with its text after the `#`. None is documentation: Python documents with
//! a docstring, a string the language reads as `__doc__`, which is a `string_literal` row and no comment.

use crate::domain::file_view::types::PythonWalk;
use crate::domain::layout_facts::service::tokens;
use crate::domain::layout_facts::types::TokenAt;

use crate::domain::node::service::{node_cell, span_at, word, written};



use crate::domain::tokens::types::Token as Lexed;
use ryo::languages;

/// Reads the `comment` rows of one Python file from its syntax tree.
pub(crate) struct CommentItem;

// The engine fills `comment` from the walked file: every row of the file, once.
ryo::fills! {
    CommentItem => languages::Comment, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Comment as languages::CatalogRelation>::Row;
        for TokenAt { id, token } in tokens(&file.syntax) {
            if token != Lexed::Comment {
                continue;
            }
            // The comment's text is what follows its `#`.
            let text = written(&file.syntax, id).0.get(1..).unwrap_or_default();
            rows.write(Row {
                id: node_cell(&file.syntax, id),
                text: word(words, languages::NameText(text)),
                file: file.syntax.file,
                span: span_at(&file.syntax, id),
                doc: languages::Flag::Lowered,
            });
        }
    }
}
