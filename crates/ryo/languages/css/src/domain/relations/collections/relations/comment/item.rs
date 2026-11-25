//! The `comment` relation of the language: every comment, with its text between `/*` and `*/` as the library read it.

use crate::domain::file_view::service::{decoded as walked_decoded, node_cell, span_at, word};
use crate::domain::file_view::types::CssWalk;
use ryo::languages::node_table;
use crate::domain::extras::types::Extra;
use crate::domain::syntax::service::{decoded, kind_bits};
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `comment` relation to, which reads its rows from a file's tree.
pub(crate) struct CommentItem;

ryo::reads! {
    CommentItem => languages::Comment |tree, at, symbols| {
        kinds: [Kind::Extra(Extra::Comment)] as kind_bits,
        given text = decoded(tree, symbols, at)?;
        row { id: node(at), text: value(text), file: file, span: span(at), doc: value(languages::Flag::Lowered) }
    }
}

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
