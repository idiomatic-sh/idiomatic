//! The `entry` relation of the language: every key of a document, with what its value is. A key is written once, where it first
//! opens its mapping or where it is the last part of a key-value: a header or a dotted key that says a key again writes no second
//! entry, and the restated key stays in `syntax_tree` as written. A key-value whose value TOML cannot read keeps its entry, and the
//! relation says the value is read in part.

use crate::domain::file_view::service::{modelled, node_cell, span_at};
use crate::domain::file_view::types::TomlWalk;
use crate::domain::model::types::Readability;
use crate::domain::walked_values::service as walked;
use crate::domain::relations::constants::UNREADABLE_VALUE;
use ryo::languages;

/// The reader the `parse` contract hands the `entry` relation to, which reads its rows from a file's tree.
pub(crate) struct EntryItem;


// The engine fills `entry` from the walked file: every key, once, with what its value is.
ryo::fills! {
    EntryItem => languages::Entry, walk(TomlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Entry as languages::CatalogRelation>::Row;
        let (model, syntax) = modelled(file);
        for fact in &model.entries {
            let value = fact.value;
            if fact.readability == Readability::Unreadable {
                rows.partial(languages::ReportText(&UNREADABLE_VALUE), languages::Missing::Any);
            }
            let text = value.map_or(languages::NO_SYMBOL, |value| walked::spanned(&syntax, words, value));
            rows.write(Row {
                decoded: walked::said(&syntax, words, value),
                key: walked::key_name(&syntax, words, fact.occurrence),
                kind: fact.kind,
                occurrence: node_cell(&syntax, fact.occurrence),
                ordinal: fact.ordinal,
                owner: node_cell(&syntax, fact.owner),
                file: syntax.file,
                span: span_at(&syntax, fact.occurrence),
                text,
                boolean: walked::boolean(&syntax, value),
                integer: walked::integer(&syntax, value),
                float: walked::decoded_as(&syntax, words, value, languages::ValueKind::Float),
                datetime: walked::decoded_as(&syntax, words, value, languages::ValueKind::Datetime),
                radix: walked::radix(&syntax, value),
            });
        }
    }
}
