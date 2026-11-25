//! The `entry` relation of the language: every module a `go.mod` requires, keyed by its path as Go reads it, with its version as
//! its value, decoded, and its spelling as its text. A line of
//! a `require` block is counted among the block's requirements, and a `require` line of its own among the file's. A go source has no
//! keys and writes no entry.

use crate::domain::extras::types::Extra;
use crate::domain::modules::service::{required_at, word_text};
use crate::domain::names::service::named;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::service::written;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `entry` relation to, which reads its rows from a file's tree.
pub(crate) struct EntryItem;

ryo::reads! {
    EntryItem => languages::Entry |tree, at, symbols| {
        kinds: [Kind::Extra(Extra::ModLine)] as kind_bits,
        given line = required_at(tree, at)?;
        given key = named(symbols, languages::NameText(&word_text(tree, line.path).0));
        given version = line.version.map_or(languages::NO_SYMBOL, |version| {
            named(symbols, languages::NameText(&word_text(tree, version).0))
        });
        given spelled = line.version.map_or(languages::NO_SYMBOL, |version| {
            named(symbols, languages::NameText(written(tree, version).0))
        });
        row {
            decoded: value(version),
            key: value(key),
            kind: value(languages::ValueKind::String),
            occurrence: node(line.path),
            ordinal: value(line.ordinal),
            owner: node(line.owner),
            file: file,
            span: span(line.path),
            text: value(spelled),
            boolean: value(languages::Flag::Lowered),
            integer: value(languages::Integer(0)),
            float: value(languages::NO_SYMBOL),
            datetime: value(languages::NO_SYMBOL),
            radix: value(languages::Integer(0)),
        }
    }
}

// The engine fills `entry` from the walked file: every module a `go.mod` requires, with its version.
ryo::fills! {
    EntryItem => languages::Entry, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Entry as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for &at in node_table::ids_of_kind(tree.table, kind_bits(Kind::Extra(Extra::ModLine))) {
            let Some(line) = crate::domain::walked::modules::required_at(&tree, at) else { continue };
            let key = crate::domain::walked::names::named(&words, languages::NameText(&crate::domain::walked::modules::word_text(&tree, line.path).0));
            let version = line.version.map_or(languages::NO_SYMBOL, |version| {
                crate::domain::walked::names::named(&words, languages::NameText(&crate::domain::walked::modules::word_text(&tree, version).0))
            });
            let spelled = line.version.map_or(languages::NO_SYMBOL, |version| {
                crate::domain::walked::names::named(&words, languages::NameText(walked_syntax::written(&tree, version).0))
            });
            rows.write(Row {
                decoded: version,
                key,
                kind: languages::ValueKind::String,
                occurrence: walked_syntax::node_cell(&tree, line.path),
                ordinal: line.ordinal,
                owner: walked_syntax::node_cell(&tree, line.owner),
                file: tree.file,
                span: walked_syntax::span_of(&tree, line.path),
                text: spelled,
                boolean: languages::Flag::Lowered,
                integer: languages::Integer(0),
                float: languages::NO_SYMBOL,
                datetime: languages::NO_SYMBOL,
                radix: languages::Integer(0),
            });
        }
    }
}
