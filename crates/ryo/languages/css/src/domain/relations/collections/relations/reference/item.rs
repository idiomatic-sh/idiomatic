//! The `reference` relation of the language: every `url(..)` outside an `@import` and an `@namespace`, by the path of the run it
//! points at (the adapter resolves the url the library decoded against the stylesheet's own path), owned by the declaration or rule it
//! is written in.

use crate::domain::file_view::service::{decoded as walked_decoded, node_cell, span_at, word};
use crate::domain::file_view::types::CssWalk;
use crate::domain::syntax::service::kind_bits;
use crate::domain::walked_targets::service::{owner as walked_owner, reference as walked_reference, url_token as walked_url_token};
use ryo::languages::node_table;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct ReferenceItem;

// The engine fills `reference` from the walked file: every `url()` and url-token that is no import's.
ryo::fills! {
    ReferenceItem => languages::Reference, walk(CssWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Reference as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        for element in [Kind::Token(Token::Url), Kind::Node(Node::Function)] {
            for &at in node_table::ids_of_kind(syntax.table, kind_bits(element)) {
                let Some(token) = walked_url_token(&syntax, at) else { continue };
                let Some(url) = walked_reference(&syntax, at) else { continue };
                let Some(target) = walked_decoded(&syntax, token) else { continue };
                rows.write(Row {
                    occurrence: node_cell(&syntax, url),
                    owner: node_cell(&syntax, walked_owner(&syntax, url)),
                    target: word(words, target),
                    file: syntax.file,
                    span: span_at(&syntax, url),
                });
            }
        }
    }
}
