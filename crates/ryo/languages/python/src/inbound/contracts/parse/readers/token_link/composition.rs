//! Optional lexical token metadata over the completed native tree only.
use super::constants::TOKEN_LINK_INCOMPLETE;
use crate::domain::file_view::types::{PythonFile, PythonWalk};
use crate::domain::layout_facts::service::{link_index, links_from, links_to, token_links};
use crate::domain::model::names::service::fixed_word;
use crate::domain::model::names::types::FixedWords;
use crate::domain::model::view::types::PartState;
use crate::domain::node::service::{keyed_id, node_cell, span_at};
use crate::domain::node::types::SyntaxFile;
use crate::domain::token_links::types::Pairing;
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
    token_links::{
        service::{link_kind, scan},
        types::{Event, TokenLink},
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

/// One row of the relation: a link between two tokens, its kind written by its handle.
fn link_row(
    syntax: &SyntaxFile<'_>,
    words: &mut languages::FileWords,
    memo: &mut FixedWords,
    link: TokenLink,
) -> languages::TokenLinkRow {
    languages::TokenLinkRow {
        source: node_cell(syntax, link.source),
        target: node_cell(syntax, link.target),
        link_kind: fixed_word(words, memo, link_kind(link.kind)),
        file: syntax.file,
        span: span_at(syntax, link.source),
    }
}

/// What each token of the file is linked to, made by the first fill of the links of one token; none, with the relation said
/// partly read when the pairing of the brackets was lost, for a key that names no element of this file.
fn indexed(
    file: &mut PythonFile<'_>,
    key: languages::CellWord,
    rows: &mut dyn languages::Rows<languages::TokenLinkRow>,
) -> Option<languages::Position> {
    if file.analysed.links == PartState::Pending {
        link_index(
            &file.syntax,
            &mut file.buffers.brackets,
            &mut file.buffers.links,
        );
        file.analysed.links = PartState::Ready;
    }
    if file.buffers.links.pairing == Pairing::Lost {
        rows.partial(
            languages::ReportText(TOKEN_LINK_INCOMPLETE),
            languages::Missing::Any,
        );
    }
    keyed_id(file.syntax.file, key)
}

// The engine fills `token-link` from the walked file: every row of the file in one pass over its tokens, or the links from or
// to one token, read from what each token is linked to.
ryo::fills! {
    TokenLinkItem => languages::TokenLink, walk(PythonWalk) |file, words, rows| {
        let syntax = file.syntax;
        let memo = &mut file.model.words.get_mut().fixed;
        let pairing = token_links(&syntax, &mut file.buffers.brackets, &mut |link| {
            rows.write(link_row(&syntax, words, memo, link));
        });
        if pairing == Pairing::Lost {
            rows.partial(languages::ReportText(TOKEN_LINK_INCOMPLETE), languages::Missing::Any);
        }
    }
    key(languages::Column::TokenLink(languages::TokenLinkColumn::Source)) |file, words, key, rows| {
        let Some(id) = indexed(file, key, rows) else {
            return;
        };
        let (syntax, memo) = (file.syntax, &mut file.model.words.get_mut().fixed);
        links_from(&file.buffers.links, id, &mut |link| rows.write(link_row(&syntax, words, memo, link)));
    }
    key(languages::Column::TokenLink(languages::TokenLinkColumn::Target)) |file, words, key, rows| {
        let Some(id) = indexed(file, key, rows) else {
            return;
        };
        let (syntax, memo) = (file.syntax, &mut file.model.words.get_mut().fixed);
        links_to(&file.buffers.links, id, &mut |link| rows.write(link_row(&syntax, words, memo, link)));
    }
}
