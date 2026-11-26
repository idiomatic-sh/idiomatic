//! Exhaustive mappings of the parser's native closed DOM and tokenizer variants.
use crate::domain::{nodes::types::Node, tokens::types::Token};
use swc_html_ast::{Child, Token as NativeToken};


/// Every native DOM child has one corresponding node kind.
pub(super) fn child_kind(child: &Child) -> Node {
    ChildSchema(child).into()
}

/// The native DOM child at its conversion boundary.
pub(super) struct ChildSchema<'a>(pub &'a Child);
impl From<ChildSchema<'_>> for Node {
    fn from(schema: ChildSchema<'_>) -> Self {
        match schema.0 {
            Child::DocumentType(_) => Node::DocumentType,
            Child::Element(_) => Node::Element,
            Child::Text(_) => Node::Text,
            Child::Comment(_) => Node::Comment,
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

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
