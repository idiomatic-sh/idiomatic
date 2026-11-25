//! The `string_literal` relation of the language: every string value, as the library decoded it, in its place. The name of a member is
//! a key, not a string literal: `entry` reads it.

use crate::domain::file_view::service::{node_cell, parent as walked_parent, span_at, word};
use crate::domain::file_view::types::JsonWalk;
use crate::domain::walked_values::service::keyness as walked_keyness;
use ryo::languages::node_table;
use crate::domain::model::types::Model;
use crate::domain::relations::constants::UNDECODED;
use crate::domain::syntax::service::{decoded, keyness, kind_bits, parent};
use crate::domain::syntax::types::{Keyness, Kind};
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `string_literal` relation to, which reads its rows from a file's tree.
pub(crate) struct StringLiteralItem;

ryo::reads! {
    StringLiteralItem => languages::StringLiteral, model(Model) |tree, model, at, symbols| {
        kinds: [Kind::Token(Token::String)] as kind_bits,
        require (keyness(tree, at) == Keyness::Other);
        when decoded(tree, symbols, at) {
            Some(value) => {
                row {
                    occurrence: node(at),
                    owner: node(parent(tree, at)),
                    value: value(value),
                    file: file,
                    span: span(at),
                    import: value(languages::Flag::Lowered),
                }
            }
            None => {
                partial UNDECODED => languages::Missing::Any;
            }
        }
    }
}

// The engine fills `string_literal` from the walked file: every string that is no member's name, as the library decoded it.
ryo::fills! {
    StringLiteralItem => languages::StringLiteral, walk(JsonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StringLiteral as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Token(Token::String))) {
            if walked_keyness(&syntax, id) != Keyness::Other {
                continue;
            }
            match node_table::decoded_of(syntax.table, id) {
                Some(value) => rows.write(Row {
                    occurrence: node_cell(&syntax, id),
                    owner: node_cell(&syntax, walked_parent(&syntax, id)),
                    value: word(words, value.0),
                    file: syntax.file,
                    span: span_at(&syntax, id),
                    import: languages::Flag::Lowered,
                }),
                None => rows.partial(languages::ReportText(&UNDECODED), languages::Missing::Any),
            }
        }
    }
}
