//! Optional lexical token metadata over the completed native tree only.
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

pub(super) struct TokenLinkItem;

ryo::reads! {
    TokenLinkItem => languages::TokenLink, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        each (scan(tree)) as event {
            when identity(event) {
                Event::Link(link) => {
                    given name = languages::intern_derived(symbols, tree.source, link_kind(link.kind));
                    row { source: node(link.source), target: node(link.target), link_kind: value(name), file: file, span: span(link.source), }
                }
                Event::Incomplete => { partial "python-token-link-incomplete" => languages::Missing::Any; }
            }
        }
    }
}

#[cfg(test)]
mod tests;
