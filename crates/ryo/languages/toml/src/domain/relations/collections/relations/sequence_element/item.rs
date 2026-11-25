//! The `sequence_element` relation of the language: every value of an array, and every member of an array of tables, in its place:
//! the two spellings of a sequence give the same rows.

use crate::domain::file_view::service::{modelled, node_cell, span_at};
use crate::domain::file_view::types::TomlWalk;
use crate::domain::walked_values::service as walked;
use crate::domain::model::service::element_at;
use crate::domain::model::service::{boolean, decoded_as, integer, radix, said};
use crate::domain::model::types::Model;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, spanned};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `sequence_element` relation to, which reads its rows from a file's tree.
pub(crate) struct SequenceElementItem;

ryo::reads! {
    SequenceElementItem => languages::SequenceElement, model(Model) |tree, model, at, symbols| {
        kinds: [
            Kind::Token(Token::BasicString),
            Kind::Token(Token::LiteralString),
            Kind::Token(Token::MlBasicString),
            Kind::Token(Token::MlLiteralString),
            Kind::Token(Token::Integer),
            Kind::Token(Token::Float),
            Kind::Token(Token::Boolean),
            Kind::Token(Token::DateTime),
            Kind::Node(Node::Array),
            Kind::Node(Node::InlineTable),
            Kind::Node(Node::ArrayTable),
        ] as kind_bits,
        given fact = element_at(model, at)?;
        given text = spanned(tree, symbols, at);
        given decoded = said(tree, symbols, Some(at));
        given float = decoded_as(tree, symbols, Some(at), languages::ValueKind::Float);
        given datetime = decoded_as(tree, symbols, Some(at), languages::ValueKind::Datetime);
        row {
            decoded: value(decoded),
            kind: value(fact.kind),
            occurrence: node(at),
            ordinal: value(fact.ordinal),
            owner: node(fact.owner),
            file: file,
            span: span(at),
            text: value(text),
            boolean: value(boolean(tree, Some(at))),
            integer: value(integer(tree, Some(at))),
            float: value(float),
            datetime: value(datetime),
            radix: value(radix(tree, Some(at))),
        }
    }
}

// The engine fills `sequence_element` from the walked file: every value of an array and every member of an array of tables.
ryo::fills! {
    SequenceElementItem => languages::SequenceElement, walk(TomlWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::SequenceElement as languages::CatalogRelation>::Row;
        let (model, syntax) = modelled(file);
        for fact in &model.elements {
            let at = Some(fact.occurrence);
            rows.write(Row {
                decoded: walked::said(&syntax, words, at),
                kind: fact.kind,
                occurrence: node_cell(&syntax, fact.occurrence),
                ordinal: fact.ordinal,
                owner: node_cell(&syntax, fact.owner),
                file: syntax.file,
                span: span_at(&syntax, fact.occurrence),
                text: walked::spanned(&syntax, words, fact.occurrence),
                boolean: walked::boolean(&syntax, at),
                integer: walked::integer(&syntax, at),
                float: walked::decoded_as(&syntax, words, at, languages::ValueKind::Float),
                datetime: walked::decoded_as(&syntax, words, at, languages::ValueKind::Datetime),
                radix: walked::radix(&syntax, at),
            });
        }
    }
}
