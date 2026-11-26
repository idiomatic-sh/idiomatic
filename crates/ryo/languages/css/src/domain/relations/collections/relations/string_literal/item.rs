//! The `string_literal` relation of the language: every string-token, as the library decoded it (its quotes and escapes resolved),
//! owned by the declaration or rule it is written in. A url-token is no string: `reference` reads the path it points at. The string
//! an `@import` names its stylesheet by is an import specifier.

use crate::domain::file_view::service::{decoded as walked_decoded, node_cell, span_at, word};
use crate::domain::file_view::types::CssWalk;
use crate::domain::syntax::service::kind_bits;
use crate::domain::walked_targets::service::{owner as walked_owner, specifier as walked_specifier};
use ryo::languages::node_table;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct StringLiteralItem;

// The engine fills `string_literal` from the walked file: every string the library decoded.
ryo::fills! {
    StringLiteralItem => languages::StringLiteral, walk(CssWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StringLiteral as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        for &id in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Token(Token::String))) {
            let Some(value) = walked_decoded(&syntax, id) else { continue };
            rows.write(Row {
                occurrence: node_cell(&syntax, id),
                owner: node_cell(&syntax, walked_owner(&syntax, id)),
                value: word(words, value),
                file: syntax.file,
                span: span_at(&syntax, id),
                import: walked_specifier(&syntax, id),
            });
        }
    }
}
