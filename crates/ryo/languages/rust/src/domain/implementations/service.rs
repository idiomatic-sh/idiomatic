//! The `implementations` service: free functions named by verb that read what an `impl` block declares, named by its header, with no
//! IO and no ports.

use super::constants::OMITTED;
use super::types::{Header, Opening};
use crate::domain::nodes::types::RustNode;
use crate::domain::syntax::service::{child, end, is_token, kind, push_token, start};
use crate::domain::syntax::types::{Gap, Kind, Leaf};
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Tree};

/// Pushes the header of the `impl` under `at` onto the name being made, token by token: what is written after the `impl` keyword up to
/// its body, its attributes, generic parameter lists, generic argument lists and where clause left out, the white space and comments
/// between two tokens one space.
fn header(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
    opened: &mut Opening,
    gap: &mut Gap,
) {
    for each in languages::children(tree.elements, at) {
        let element = kind(tree, each);
        if element == Kind::Node(RustNode::AssocItemList) {
            *opened = Opening::Body;
            return;
        }
        if *opened == Opening::Body {
            return;
        }
        if OMITTED.contains(&element) {
            continue;
        }
        if is_token(tree, each) == Leaf::Node {
            header(tree, symbols, each, opened, gap);
            continue;
        }
        if *opened == Opening::Before {
            if element == Kind::Token(Token::ImplKw) {
                *opened = Opening::Header;
            }
            continue;
        }
        push_token(tree, symbols, each, gap);
    }
}

/// The header of the `impl` at `at`: its name (`Display for Point` of `impl<T: X> Display for Point<T> where T: Y`),
/// from its `impl` keyword to its body.
pub(crate) fn implemented(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> Option<Header> {
    let begun = languages::begin_name(symbols);
    let mut gap = Gap::Start;
    header(tree, symbols, at, &mut Opening::Before, &mut gap);
    let name = languages::end_name(symbols, tree.source, begun);
    if gap == Gap::Start {
        return None;
    }
    let keyword = child(tree, at, Kind::Token(Token::ImplKw))?;
    Some(Header {
        name,
        from: start(tree, keyword),
        to: child(tree, at, Kind::Node(RustNode::AssocItemList))
            .map_or_else(|| end(tree, at), |body| start(tree, body)),
    })
}
