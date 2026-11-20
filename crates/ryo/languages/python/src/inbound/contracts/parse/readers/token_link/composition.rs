//! Optional lexical token metadata over the completed native tree only.
use super::constants::TOKEN_LINK_INCOMPLETE;
use crate::domain::file_view::types::PythonWalk;
use crate::domain::layout_facts::service::token_links;
use crate::domain::node::service::{node_cell, span_at, word};
use crate::domain::token_links::types::Pairing;
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
        each (scan(tree, std::cell::RefMut::map(model.scratch.borrow_mut(), |scratch| &mut scratch.token_stack))) as event {
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

// The engine fills `token-link` from the walked file: every row of the file, once.
ryo::fills! {
    TokenLinkItem => languages::TokenLink, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::TokenLink as languages::CatalogRelation>::Row;
        let mut stack = Vec::new();
        let syntax = file.syntax;
        let pairing = token_links(&syntax, &mut stack, &mut |link| {
            rows.write(Row {
                source: node_cell(&syntax, link.source),
                target: node_cell(&syntax, link.target),
                link_kind: word(words, link_kind(link.kind)),
                file: syntax.file,
                span: span_at(&syntax, link.source),
            });
        });
        if pairing == Pairing::Lost {
            rows.partial(languages::ReportText(TOKEN_LINK_INCOMPLETE), languages::Missing::Any);
        }
    }
}
