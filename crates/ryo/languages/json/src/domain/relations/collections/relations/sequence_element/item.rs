//! The `sequence_element` relation of the language: every value of an array, in its place, `null` included (kind `null`).

use crate::domain::file_view::service::{kind as walked_kind, modelled, node_cell, parent as walked_parent, span_at};
use crate::domain::file_view::types::JsonWalk;
use crate::domain::walked_values::service as walked;
use ryo::languages::node_table;
use crate::domain::model::service::{float, place, radix};
use crate::domain::model::types::Model;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{
    boolean, holder, integer, kind, kind_bits, parent, said, spanned, value_kind,
};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `sequence_element` relation to, which reads its rows from a file's tree.
pub(crate) struct SequenceElementItem;

ryo::reads! {
    SequenceElementItem => languages::SequenceElement, model(Model) |tree, model, at, symbols| {
        kinds: [
            Kind::Token(Token::String),
            Kind::Token(Token::Number),
            Kind::Token(Token::True),
            Kind::Token(Token::False),
            Kind::Token(Token::Null),
            Kind::Node(Node::Object),
            Kind::Node(Node::Array),
        ] as kind_bits,
        given container = parent(tree, at);
        require (kind(tree, container) == Kind::Node(Node::Array));
        given text = spanned(tree, symbols, at);
        given decoded = said(tree, symbols, at);
        given kind = value_kind(tree, at)?;
        given float = float(tree, symbols, at);
        row {
            decoded: value(decoded),
            kind: value(kind),
            occurrence: node(at),
            ordinal: value(place(model, at)),
            owner: node(holder(tree, container)),
            file: file,
            span: span(at),
            text: value(text),
            boolean: value(boolean(tree, at)),
            integer: value(integer(tree, at)),
            float: value(float),
            // RFC 8259 has no date-time value.
            datetime: value(languages::NO_SYMBOL),
            radix: value(radix(tree, at)),
        }
    }
}

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
