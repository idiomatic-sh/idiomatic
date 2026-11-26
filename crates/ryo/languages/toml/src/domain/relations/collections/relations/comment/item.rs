//! The `comment` relation of the language: every comment, with its text after the `#`.

use crate::domain::file_view::service::{node_cell, span_at, word, written};
use crate::domain::file_view::types::TomlWalk;
use ryo::languages::node_table;
use crate::domain::syntax::service::{kind_bits};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `comment` relation to, which reads its rows from a file's tree.
pub(crate) struct CommentItem;


// The engine fills `comment` from the walked file: every comment, once.
ryo::fills! {
    CommentItem => languages::Comment, walk(TomlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Comment as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Token(Token::Comment))) {
            // The comment's text is what follows its `#`.
            let text = written(&syntax, id).get(1..).unwrap_or_default();
            rows.write(Row {
                id: node_cell(&syntax, id),
                text: word(words, text),
                file: syntax.file,
                span: span_at(&syntax, id),
                doc: languages::Flag::Lowered,
            });
        }
    }
}
