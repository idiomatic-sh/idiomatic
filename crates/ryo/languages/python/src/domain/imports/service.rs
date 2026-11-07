//! The `imports` service: free functions named by verb over the domain's types, with no IO and no ports. The names an import binds
//! and the paths it imports, as rows hold them.

use ryo::languages::{self, Position, Symbol, Tree};

use crate::domain::extras::types::Extra;
use crate::domain::names::constants::DOT;
use crate::domain::names::service::{push_identifier, unknown};
use crate::domain::names::types::{Imported, Unresolved};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, kind, name_of, name_token, nodes, parent};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// Adds the dots and the module of a `from` import: `..m`.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a `from` import's module is its dots and its identifier before its first alias; the other kinds under it are its keywords and white space"
)]
pub(crate) fn push_module(tree: &Tree<'_>, symbols: &mut languages::Symbols, import: Position) {
    for each in languages::children(tree.elements, import) {
        match kind(tree, each) {
            Kind::Token(Token::Dot) => languages::push_text(symbols, languages::NameText(DOT)),
            Kind::Token(Token::Ellipsis) => {
                for _ in 0_i32..3_i32 {
                    languages::push_text(symbols, languages::NameText(DOT));
                }
            }
            Kind::Extra(Extra::Identifier) => push_identifier(tree, symbols, each),
            Kind::Node(Node::Alias) => return,
            _ => {}
        }
    }
}

/// Whether the alias at `alias` is `*`, which imports every name of its module, or names what it imports.
#[must_use]
pub(crate) fn imported(tree: &Tree<'_>, alias: Position) -> Imported {
    if child(tree, alias, Kind::Extra(Extra::Identifier))
        .is_some_and(|identifier| child(tree, identifier, Kind::Token(Token::Star)).is_some())
    {
        Imported::Everything
    } else {
        Imported::Names
    }
}

/// The name an alias of an import binds: its `as` name, the first part of a module it imports, the name it imports; for `*`, the
/// unknown value of a wildcard import: the names it binds are the module's, which no file of it lists.
pub(crate) fn import_binding(tree: &Tree<'_>, alias: Position) -> Option<Symbol> {
    if imported(tree, alias) == Imported::Everything {
        return Some(unknown(Unresolved::WildcardImport));
    }
    let mut identifiers =
        nodes(tree, alias).filter(|each| kind(tree, *each) == Kind::Extra(Extra::Identifier));
    let name = identifiers.next()?;
    if let Some(asname) = identifiers.next() {
        return name_of(tree, asname);
    }
    name_of(tree, name)
}

/// The path an alias of an import imports: `a.b` for `import a.b`, `..m.x` for `from ..m import x`, `.x` for `from . import x`; for `*`,
/// the module it imports from, `m` for `from m import *`: the module is known, only the names it binds are not.
pub(crate) fn import_target(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    alias: Position,
) -> Option<Symbol> {
    let import = parent(tree, alias);
    if imported(tree, alias) == Imported::Everything {
        let from = languages::begin_name(symbols);
        push_module(tree, symbols, import);
        return Some(languages::end_name(symbols, tree.source, from));
    }
    let name = child(tree, alias, Kind::Extra(Extra::Identifier))?;
    let from = languages::begin_name(symbols);
    if kind(tree, import) == Kind::Node(Node::ImportFrom) {
        push_module(tree, symbols, import);
        if child(tree, import, Kind::Extra(Extra::Identifier)).is_some() {
            languages::push_text(symbols, languages::NameText(DOT));
        }
    }
    push_identifier(tree, symbols, name);
    Some(languages::end_name(symbols, tree.source, from))
}

/// The span of the name an alias binds: its `as` name, or the first part of the name it imports.
#[must_use]
pub(crate) fn binding_at(tree: &Tree<'_>, alias: Position) -> Position {
    let mut identifiers =
        nodes(tree, alias).filter(|each| kind(tree, *each) == Kind::Extra(Extra::Identifier));
    let name = identifiers.next();
    identifiers
        .next()
        .or_else(|| {
            let each = name?;
            name_token(tree, each)
        })
        .or(name)
        .unwrap_or(alias)
}
