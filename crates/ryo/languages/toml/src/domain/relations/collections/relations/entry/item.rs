//! The `entry` relation of the language: every key of a document, with what its value is. A key is written once, where it first
//! opens its mapping or where it is the last part of a key-value: a header or a dotted key that says a key again writes no second
//! entry, and the restated key stays in `syntax_tree` as written. A key-value whose value TOML cannot read keeps its entry, and the
//! relation says the value is read in part.

use crate::domain::keys::service::key_name;
use crate::domain::model::service::{
    boolean, decoded_as, entry_at, integer, radix, readability, said,
};
use crate::domain::model::types::Model;
use crate::domain::model::types::Readability;
use crate::domain::relations::constants::UNREADABLE_VALUE;
use crate::domain::syntax::service::{kind_bits, spanned};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `entry` relation to, which reads its rows from a file's tree.
pub(crate) struct EntryItem;

ryo::reads! {
    EntryItem => languages::Entry, model(Model) |tree, model, at, symbols| {
        kinds: [Kind::Token(Token::UnquotedKey), Kind::Token(Token::BasicString), Kind::Token(Token::LiteralString)] as kind_bits,
        given fact = entry_at(model, at)?;
        given key = key_name(tree, symbols, at);
        given value = fact.value;
        given text = value.map_or(languages::NO_SYMBOL, |value| spanned(tree, symbols, value));
        given decoded = said(tree, symbols, value);
        when readability(fact) {
            Readability::Unreadable => {
                partial UNREADABLE_VALUE => languages::Missing::Any;
            }
            Readability::Readable => {}
        }
        given float = decoded_as(tree, symbols, value, languages::ValueKind::Float);
        given datetime = decoded_as(tree, symbols, value, languages::ValueKind::Datetime);
        row {
            decoded: value(decoded),
            key: value(key),
            kind: value(fact.kind),
            occurrence: node(at),
            ordinal: value(fact.ordinal),
            owner: node(fact.owner),
            file: file,
            span: span(at),
            text: value(text),
            boolean: value(boolean(tree, value)),
            integer: value(integer(tree, value)),
            float: value(float),
            datetime: value(datetime),
            radix: value(radix(tree, value)),
        }
    }
}
