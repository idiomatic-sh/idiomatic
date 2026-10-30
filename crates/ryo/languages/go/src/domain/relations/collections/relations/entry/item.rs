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
