//! The `entry` relation of the language: every member of an object, with what its value is. The key is the name of the member, decoded;
//! the owner is the mapping the object is. A member whose value is `null` has an entry of kind `null`.

use crate::domain::file_view::service::{modelled, node_cell, parent as walked_parent, span_at};
use crate::domain::file_view::types::JsonWalk;
use crate::domain::walked_values::service as walked;
use ryo::languages::node_table;
use crate::domain::model::service::{float, place, radix};
use crate::domain::model::types::Model;
use crate::domain::syntax::service::{
    boolean, decoded_name, integer, keyness, kind_bits, mapping_id, parent, said, spanned,
    value_kind, value_of,
};
use crate::domain::syntax::types::{Keyness, Kind};
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `entry` relation to, which reads its rows from a file's tree.
pub(crate) struct EntryItem;

ryo::reads! {
    EntryItem => languages::Entry, model(Model) |tree, model, at, symbols| {
        kinds: [Kind::Token(Token::String)] as kind_bits,
        require (keyness(tree, at) == Keyness::Key);
        given member = parent(tree, at);
        given value = value_of(tree, member)?;
        given key = decoded_name(tree, symbols, at);
        given text = spanned(tree, symbols, value);
        given decoded = said(tree, symbols, value);
        given kind = value_kind(tree, value)?;
        given float = float(tree, symbols, value);
        row {
            decoded: value(decoded),
            key: value(key),
            kind: value(kind),
            occurrence: node(at),
            ordinal: value(place(model, member)),
            owner: node(mapping_id(tree, parent(tree, member))),
            file: file,
            span: span(at),
            text: value(text),
            boolean: value(boolean(tree, value)),
            integer: value(integer(tree, value)),
            float: value(float),
            // RFC 8259 has no date-time value.
            datetime: value(languages::NO_SYMBOL),
            radix: value(radix(tree, value)),
        }
    }
}

// The engine fills `entry` from the walked file: the name of every member, with what its value is.
ryo::fills! {
    EntryItem => languages::Entry, walk(JsonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Entry as languages::CatalogRelation>::Row;
        let (model, syntax) = modelled(file);
        for &at in node_table::ids_of_kind(syntax.table, kind_bits(Kind::Token(Token::String))) {
            if walked::keyness(&syntax, at) != Keyness::Key {
                continue;
            }
            let member = walked_parent(&syntax, at);
            let Some(value) = walked::value_of(&syntax, member) else { continue };
            let Some(kind) = walked::value_kind(&syntax, value) else { continue };
            rows.write(Row {
                decoded: walked::said(&syntax, words, value),
                key: walked::decoded_name(&syntax, words, at),
                kind,
                occurrence: node_cell(&syntax, at),
                ordinal: walked::place(model, member),
                owner: node_cell(&syntax, walked::mapping_id(&syntax, walked_parent(&syntax, member))),
                file: syntax.file,
                span: span_at(&syntax, at),
                text: walked::spanned(&syntax, words, value),
                boolean: walked::boolean(&syntax, value),
                integer: walked::integer(&syntax, value),
                float: walked::float(&syntax, words, value),
                datetime: languages::NO_SYMBOL,
                radix: walked::radix(&syntax, value),
            });
        }
    }
}
