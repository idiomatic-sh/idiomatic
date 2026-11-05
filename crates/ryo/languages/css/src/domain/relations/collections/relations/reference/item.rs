//! The `reference` relation of the language: every `url(..)` outside an `@import` and an `@namespace`, by the path of the run it
//! points at (the adapter resolves the url the library decoded against the stylesheet's own path), owned by the declaration or rule it
//! is written in.

use crate::domain::nodes::types::CssNode;
use crate::domain::syntax::service::{decoded, kind_bits};
use crate::domain::syntax::types::Kind;
use crate::domain::targets::service::{owner, reference, url_token};
use crate::domain::tokens::types::Token;
use ryo::languages;

/// The reader the `parse` contract hands the `reference` relation to, which reads its rows from a file's tree.
pub(crate) struct ReferenceItem;

ryo::reads! {
    ReferenceItem => languages::Reference |tree, at, symbols| {
        kinds: [Kind::Token(Token::Url), Kind::Node(CssNode::Function)] as kind_bits,
        given token = url_token(tree, at)?;
        given url = reference(tree, at)?;
        given target = decoded(tree, symbols, token)?;
        row {
            occurrence: node(url),
            owner: node(owner(tree, url)),
            target: value(target),
            file: file,
            span: span(url),
        }
    }
}
