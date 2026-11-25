//! The visibilities of the TypeScript tree: `export` and `export default` on a declaration, and an accessibility on a class member.
use super::{
    constants::{ACCESSIBILITIES, SPACE},
    types::{Declaring, Visible},
};
use crate::domain::names::service::{declared, nodes, text};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{service::kind, types::Kind};
use crate::domain::tokens::types::Token;
use crate::domain::syntax::reading::{Interner, Syntax};
use ryo::languages::{self, Position, Span, Symbol};

/// Whether the node at `at` has a `defines` row of its own.
fn defined(tree: &impl Syntax, at: Position) -> Declaring {
    if declared(tree, at).next().is_some() {
        Declaring::Declares
    } else {
        Declaring::Silent
    }
}

/// Whether the node at `at` is a declaration an export can hold, with a `defines` row of its own.
fn declaration(tree: &impl Syntax, at: Position) -> Declaring {
    let declaration = matches!(
        kind(tree, at),
        Kind::Node(
            Node::FunctionDeclaration
                | Node::TSDeclareFunction
                | Node::ClassDeclaration
                | Node::TSInterfaceDeclaration
                | Node::TSTypeAliasDeclaration
                | Node::TSEnumDeclaration
                | Node::TSModuleDeclaration
        )
    );
    if declaration {
        defined(tree, at)
    } else {
        Declaring::Silent
    }
}

/// The declarations an export holds, each by its own node: the declarators of a variable declaration, or the one declaration.
fn exported<'a>(tree: &'a impl Syntax, at: Position) -> impl Iterator<Item = Position> + 'a {
    let held = nodes(tree, at).find(|child| kind(tree, *child) != Kind::Node(Node::Decorator));
    let variables = held.filter(|each| kind(tree, *each) == Kind::Node(Node::VariableDeclaration));
    let own =
        held.filter(|each| variables.is_none() && declaration(tree, *each) == Declaring::Declares);
    own.into_iter()
        .chain(variables.into_iter().flat_map(move |list| {
            nodes(tree, list).filter(move |each| {
                kind(tree, *each) == Kind::Node(Node::VariableDeclarator)
                    && defined(tree, *each) == Declaring::Declares
            })
        }))
}

/// The keyword tokens of the node at `at` that write its visibility: those before its declaration for an export, the accessibility of a
/// class member.
fn keywords<'a>(tree: &'a impl Syntax, at: Position) -> impl Iterator<Item = Position> + 'a {
    let export = matches!(
        kind(tree, at),
        Kind::Node(Node::ExportNamedDeclaration | Node::ExportDefaultDeclaration)
    );
    let held = nodes(tree, at)
        .find(|child| kind(tree, *child) != Kind::Node(Node::Decorator))
        .map_or_else(
            || tree.end_of(at),
            |child| tree.start_of(child),
        );
    tree.children_of(at).filter(move |child| {
        kind(tree, *child) == Kind::Token(Token::Keyword)
            && if export {
                tree.end_of(*child) <= held
            } else {
                ACCESSIBILITIES.contains(&text(tree, *child).0)
            }
    })
}

/// The visibilities the node at `at` writes, one for each declaration it is written on.
pub(crate) fn visible<'a>(tree: &'a impl Syntax, at: Position) -> impl Iterator<Item = Visible> + 'a {
    let first = keywords(tree, at).next();
    let last = keywords(tree, at).last();
    let export = matches!(
        kind(tree, at),
        Kind::Node(Node::ExportNamedDeclaration | Node::ExportDefaultDeclaration)
    );
    let member = (!export && defined(tree, at) == Declaring::Declares).then_some(at);
    let declarations = export
        .then(|| exported(tree, at))
        .into_iter()
        .flatten()
        .chain(member);
    let pair = first.zip(last);
    declarations.filter_map(move |declaration| {
        pair.map(|(opening, closing)| Visible {
            holder: at,
            occurrence: opening,
            declaration,
            from: tree.start_of(opening),
            to: tree.end_of(closing),
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
pub(crate) fn written(tree: &impl Syntax, symbols: &mut impl Interner, visible: &Visible) -> Symbol {
    let begun = symbols.begin_name();
    for (place, keyword) in keywords(tree, visible.holder).enumerate() {
        if place > 0 {
            symbols.push_text(languages::NameText(SPACE));
        }
        symbols.push_span(tree.bytes(), tree.start_of(keyword), tree.end_of(keyword));
    }
    symbols.end_name(tree.bytes(), begun)
}
