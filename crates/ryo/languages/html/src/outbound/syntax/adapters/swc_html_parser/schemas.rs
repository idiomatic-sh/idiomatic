//! Exhaustive mappings of the parser's native closed DOM and tokenizer variants.
use crate::domain::{nodes::types::HtmlNode, tokens::types::Token};
use swc_html_ast::{Child, Token as NativeToken};

use super::types::{PropertyValue, Published};
use crate::domain::syntax::service::property_id;

/// Every native DOM child has one corresponding node kind.
pub(super) fn child_kind(child: &Child) -> HtmlNode {
    ChildSchema(child).into()
}

/// The native DOM child at its conversion boundary.
pub(super) struct ChildSchema<'a>(pub &'a Child);
impl From<ChildSchema<'_>> for HtmlNode {
    fn from(schema: ChildSchema<'_>) -> Self {
        match schema.0 {
            Child::DocumentType(_) => HtmlNode::DocumentType,
            Child::Element(_) => HtmlNode::Element,
            Child::Text(_) => HtmlNode::Text,
            Child::Comment(_) => HtmlNode::Comment,
        }
    }
}

/// EOF has no source bytes; every other tokenizer variant has a token kind.
pub(super) fn token_kind(token: &NativeToken) -> Option<Token> {
    match token {
        NativeToken::Doctype { .. } => Some(Token::Doctype),
        NativeToken::StartTag { .. } => Some(Token::StartTag),
        NativeToken::EndTag { .. } => Some(Token::EndTag),
        NativeToken::Comment { .. } => Some(Token::Comment),
        NativeToken::Character { .. } => Some(Token::Characters),
        NativeToken::Eof => None,
    }
}

/// The properties of a node as the one JSON text the adapter writes them in: each property's number and its value, in order.
pub(super) fn published(values: &[PropertyValue<'_>]) -> Published {
    Published(
        serde_json::Value::Array(
            values
                .iter()
                .map(|PropertyValue { key, value }| {
                    serde_json::json!([property_id(*key).0, value.0])
                })
                .collect(),
        )
        .to_string(),
    )
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
