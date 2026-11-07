//! The visibilities of the JavaScript tree: `export` and `export default` on a declaration, each written on the declaration's node.
use super::constants::SPACE;
use super::types::{Declaring, Visible};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, nodes};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Position, Span, Symbol, Symbols, Tree};

/// Whether the node at `at` declares a name of its own: a named function or class, or a declarator.
fn declares(tree: &Tree<'_>, at: Position) -> Declaring {
    let declares = match kind(tree, at) {
        Kind::Node(Node::FunctionDeclaration | Node::ClassDeclaration) => {
            nodes(tree, at).any(|child| kind(tree, child) == Kind::Node(Node::Identifier))
        }
        Kind::Node(Node::VariableDeclarator) => true,
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => false,
    };
    if declares {
        Declaring::Declares
    } else {
        Declaring::Silent
    }
}

/// The declarations an export holds, each by its own node: the declarators of a variable declaration, or the one declaration.
fn exported<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    let held = nodes(tree, at).next();
    let variables = held.filter(|each| kind(tree, *each) == Kind::Node(Node::VariableDeclaration));
    let own =
        held.filter(|each| variables.is_none() && declares(tree, *each) == Declaring::Declares);
    own.into_iter()
        .chain(variables.into_iter().flat_map(move |list| {
            nodes(tree, list).filter(move |each| declares(tree, *each) == Declaring::Declares)
        }))
}

/// The keyword tokens of the export at `at` before its declaration: `export`, and `default` when it is one.
fn keywords<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    let held = nodes(tree, at).next().map_or_else(
        || languages::end_at(tree.elements, at),
        |child| languages::start_at(tree.elements, child),
    );
    languages::children(tree.elements, at).filter(move |child| {
        kind(tree, *child) == Kind::Token(Token::Keyword)
            && languages::end_at(tree.elements, *child) <= held
    })
}

/// The visibilities the export at `at` writes, one for each declaration it holds.
pub(crate) fn visible<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Visible> + 'a {
    let pair = keywords(tree, at).next().zip(keywords(tree, at).last());
    exported(tree, at).filter_map(move |declaration| {
        pair.map(|(first, last)| Visible {
            holder: at,
            occurrence: first,
            declaration,
            from: languages::start_at(tree.elements, first),
            to: languages::end_at(tree.elements, last),
        })
    })
}

/// Where a visibility is written.
pub(crate) fn span(visible: &Visible) -> Span {
    Span {
        start: visible.from,
        end: visible.to,
    }
}

/// A visibility as written, its keywords one space apart.
pub(crate) fn written(tree: &Tree<'_>, symbols: &mut Symbols, visible: &Visible) -> Symbol {
    let begun = languages::begin_name(symbols);
    for (place, keyword) in keywords(tree, visible.holder).enumerate() {
        if place > 0 {
            languages::push_text(symbols, languages::NameText(SPACE));
        }
        languages::push_span(
            symbols,
            tree.source,
            languages::start_at(tree.elements, keyword),
            languages::end_at(tree.elements, keyword),
        );
    }
    languages::end_name(symbols, tree.source, begun)
}
