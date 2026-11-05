//! The library's own tree of a file, in the language's kinds: what the shape rail of the conformance holds our tree to. It parses the
//! text with the library and gathers each node and token the library gives, at its own range; it reads nothing of the adapter, its
//! schemas or the element table, so a tree the adapter writes wrongly is a difference and not the same mistake twice. The language's
//! kind of a kind of the library is found here by its name alone: the language names a kind as the library does, in kebab case.

use ra_ap_syntax::{Edition, NodeOrToken, SourceFile, SyntaxKind, SyntaxNode, SyntaxToken};
use ryo::languages::Offset;
use ryo::languages::contract::{Leafness, Walked};
use strum::IntoEnumIterator as _;

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::RustNode;
use crate::domain::syntax::service::{kind_bits, kind_name};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// The language's kind for the library's `kind`: the one named as the library names it, lower case, `-` for `_`.
fn named(kind: SyntaxKind) -> Kind {
    let name = format!("{kind:?}").to_lowercase().replace('_', "-");
    RustNode::iter()
        .map(Kind::Node)
        .chain(Token::iter().map(Kind::Token))
        .find(|each| kind_name(*each).0 == name)
        .unwrap_or(Kind::Extra(Extra::Other))
}

/// The offset a position of the library's text stands for.
fn offset(position: ra_ap_syntax::TextSize) -> Offset {
    Offset(u32::from(position))
}

/// A token of the library's tree.
fn token(token: &SyntaxToken) -> Walked {
    let range = token.text_range();
    Walked {
        kind: kind_bits(named(token.kind())),
        leaf: Leafness::Token,
        start: offset(range.start()),
        end: offset(range.end()),
        children: Vec::new(),
    }
}

/// A node of the library's tree, with everything under it.
fn node(node: &SyntaxNode) -> Walked {
    let range = node.text_range();
    Walked {
        kind: kind_bits(named(node.kind())),
        leaf: Leafness::Node,
        start: offset(range.start()),
        end: offset(range.end()),
        children: node
            .children_with_tokens()
            .map(|each| match each {
                NodeOrToken::Node(inner) => self::node(&inner),
                NodeOrToken::Token(inner) => token(&inner),
            })
            .collect(),
    }
}

/// The library's tree of `text`: parsed by the library, each node and token gathered with its range.
#[must_use]
pub(crate) fn walked(text: &str) -> Walked {
    node(&SourceFile::parse(text, Edition::CURRENT).syntax_node())
}
