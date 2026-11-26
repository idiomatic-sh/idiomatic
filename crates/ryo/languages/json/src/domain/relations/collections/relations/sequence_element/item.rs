//! The `sequence_element` relation of the language: every value of an array, in its place, `null` included (kind `null`).

use crate::domain::file_view::service::{kind as walked_kind, modelled, node_cell, parent as walked_parent, span_at};
use crate::domain::file_view::types::JsonWalk;
use crate::domain::walked_values::service as walked;
use ryo::languages::node_table;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The item the engine's fill of the relation is registered on.
pub(crate) struct SequenceElementItem;

// The engine fills `sequence_element` from the walked file: every value of an array.
ryo::fills! {
    SequenceElementItem => languages::SequenceElement, walk(JsonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::SequenceElement as languages::CatalogRelation>::Row;
        let (model, syntax) = modelled(file);
        for element in [
            Kind::Token(Token::String),
            Kind::Token(Token::Number),
            Kind::Token(Token::True),
            Kind::Token(Token::False),
            Kind::Token(Token::Null),
            Kind::Node(Node::Object),
            Kind::Node(Node::Array),
        ] {
            for &at in node_table::ids_of_kind(syntax.table, kind_bits(element)) {
                let container = walked_parent(&syntax, at);
                if walked_kind(&syntax, container) != Kind::Node(Node::Array) {
                    continue;
                }
                let Some(kind) = walked::value_kind(&syntax, at) else { continue };
                rows.write(Row {
                    decoded: walked::said(&syntax, words, at),
                    kind,
                    occurrence: node_cell(&syntax, at),
                    ordinal: walked::place(model, at),
                    owner: node_cell(&syntax, walked::holder(&syntax, container)),
                    file: syntax.file,
                    span: span_at(&syntax, at),
                    text: walked::spanned(&syntax, words, at),
                    boolean: walked::boolean(&syntax, at),
                    integer: walked::integer(&syntax, at),
                    float: walked::float(&syntax, words, at),
                    datetime: languages::NO_SYMBOL,
                    radix: walked::radix(&syntax, at),
                });
            }
        }
    }
}
