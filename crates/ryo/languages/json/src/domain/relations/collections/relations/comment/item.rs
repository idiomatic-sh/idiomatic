//! The `comment` relation of the language: every comment of a file read as json with comments, with its text after the `//` or between
//! the `/*` and the `*/`.

use crate::domain::file_view::service::{node_cell, span_at};
use crate::domain::file_view::types::JsonWalk;
use crate::domain::walked_values::service as walked;
use ryo::languages::node_table;
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

// The engine fills `comment` from the walked file: every comment, once.
ryo::fills! {
    CommentItem => languages::Comment, walk(JsonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Comment as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        for extra in [Extra::LineComment, Extra::BlockComment] {
            for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Extra(extra))) {
                rows.write(Row {
                    id: node_cell(&syntax, id),
                    text: walked::comment_text(&syntax, words, id),
                    file: syntax.file,
                    span: span_at(&syntax, id),
                    doc: languages::Flag::Lowered,
                });
            }
        }
    }
}
