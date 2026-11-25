//! The `static_import` relation of the language: every `@import`, by the path of the run it imports (the adapter resolves the url the
//! library decoded against the stylesheet's own path). An `@import`
//! brings in every rule of the stylesheet it names and binds no name it could list: its binding is the unknown value of that.

use crate::domain::file_view::service::{decoded as walked_decoded, node_cell, span_at, word};
use crate::domain::file_view::types::CssWalk;
use crate::domain::walked_targets::service::{import_token as walked_import_token, owner as walked_owner};
use ryo::languages::node_table;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{decoded, kind_bits};
use crate::domain::syntax::types::Kind;
use crate::domain::targets::service::{import_token, owner};
use ryo::languages;

/// The reader the `parse` contract hands the `static_import` relation to, which reads its rows from a file's tree.
pub(crate) struct StaticImportItem;

/// The number of `imports-every-rule` in the language's list of reasons, `UNKNOWN_REASONS`.
const EVERY_RULE: languages::UnknownReason = languages::UnknownReason(48);

ryo::reads! {
    StaticImportItem => languages::StaticImport |tree, at, symbols| {
        kinds: [Kind::Node(Node::AtRule)] as kind_bits,
        given token = import_token(tree, at)?;
        given target = decoded(tree, symbols, token)?;
        row {
            occurrence: node(at),
            owner: node(owner(tree, at)),
            binding: value(languages::unknown_symbol(EVERY_RULE)),
            target: value(target),
            file: file,
            span: span(at),
        }
    }
}

// The engine fills `static_import` from the walked file: every `@import` that names a target.
ryo::fills! {
    StaticImportItem => languages::StaticImport, walk(CssWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StaticImport as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        for &at in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Node(Node::AtRule))) {
            let Some(token) = walked_import_token(&syntax, at) else { continue };
            let Some(target) = walked_decoded(&syntax, token) else { continue };
            rows.write(Row {
                occurrence: node_cell(&syntax, at),
                owner: node_cell(&syntax, walked_owner(&syntax, at)),
                binding: languages::unknown_symbol(EVERY_RULE),
                target: word(words, target),
                file: syntax.file,
                span: span_at(&syntax, at),
            });
        }
    }
}
