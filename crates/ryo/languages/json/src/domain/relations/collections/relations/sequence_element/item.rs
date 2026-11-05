//! The `sequence_element` relation of the language: every value of an array, in its place, `null` included (kind `null`).

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
