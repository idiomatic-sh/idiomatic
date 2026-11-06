//! Optional lexical token metadata over the completed native tree only.
use super::constants::TOKEN_LINK_INCOMPLETE;
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
    token_links::{
        service::{link_kind, scan},
        types::Event,
    },
};
use ryo::languages;
use std::convert::identity;

/// Reads the optional lexical links between physical native tokens.
pub(crate) struct TokenLinkItem;

ryo::reads! {
    TokenLinkItem => languages::TokenLink, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        each (scan(tree)) as event {
            when identity(event) {
                Event::Link(link) => {
                    given name = languages::intern_derived(symbols, tree.source, link_kind(link.kind));
                    row { source: node(link.source), target: node(link.target), link_kind: value(name), file: file, span: span(link.source), }
                }
                Event::Incomplete => { partial TOKEN_LINK_INCOMPLETE => languages::Missing::Any; }
            }
        }
    }
}
