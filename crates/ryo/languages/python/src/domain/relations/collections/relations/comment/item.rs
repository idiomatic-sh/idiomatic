//! The `comment` relation of the language: every comment, with its text after the `#`. None is documentation: Python documents with
//! a docstring, a string the language reads as `__doc__`, which is a `string_literal` row and no comment.

use crate::domain::file_view::types::PythonWalk;
use crate::domain::layout_facts::service::token_ids;
use crate::domain::model::types::Declarations;
use crate::domain::node::service::{node_cell, span_at, word, written};
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

// The engine fills `comment` from the walked file: every row of the file, once.
ryo::fills! {
    CommentItem => languages::Comment, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Comment as languages::CatalogRelation>::Row;
        let text_column = <languages::Comment as languages::CatalogRelation>::columns().iter()
            .position(|column| *column == languages::CommentColumn::Text).expect("catalog column");
        let wants_text = rows.wants_column(text_column);
        for &id in token_ids(&file.syntax, Token::Comment) {
            let text = if wants_text {
                let written = written(&file.syntax, id).0.get(1..).unwrap_or_default();
                word(words, languages::NameText(written))
            } else {
                languages::Symbol(0)
            };
            rows.write(Row {
                id: node_cell(&file.syntax, id),
                text,
                file: file.syntax.file,
                span: span_at(&file.syntax, id),
                doc: languages::Flag::Lowered,
            });
        }
    }
}
