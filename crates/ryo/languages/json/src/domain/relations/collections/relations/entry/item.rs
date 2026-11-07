//! The `entry` relation of the language: every member of an object, with what its value is. The key is the name of the member, decoded;
//! the owner is the mapping the object is. A member whose value is `null` has an entry of kind `null`.

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
