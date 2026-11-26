//! The `comment` relation of the language: every comment, with its text between `/*` and `*/` as the library read it.

use crate::domain::file_view::service::{decoded as walked_decoded, node_cell, span_at, word};
use crate::domain::file_view::types::CssWalk;
use crate::domain::syntax::service::kind_bits;
use ryo::languages::node_table;
use crate::domain::extras::types::Extra;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct CommentItem;

// The engine fills `comment` from the walked file: every comment the library decoded.
ryo::fills! {
    CommentItem => languages::Comment, walk(CssWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Comment as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Extra(Extra::Comment))) {
            let Some(text) = walked_decoded(&syntax, id) else { continue };
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
