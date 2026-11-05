//! The bindings an import declaration makes and the module it names, read from the TypeScript syntax tree.

use crate::domain::names::constants::REQUIRE;
use crate::domain::names::service::{identifier, nodes, string, text, unknown};
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::{service::kind, types::Kind};
use ryo::languages::{Position, Symbol, Symbols, Tree};

/// Each imported binding; a side-effect import has one unknown binding.
pub(crate) fn imports<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    let bound = move |child: &Position| {
        matches!(
            kind(tree, *child),
            Kind::Node(
                TypeScriptNode::ImportSpecifier
                    | TypeScriptNode::ImportDefaultSpecifier
                    | TypeScriptNode::ImportNamespaceSpecifier
            )
        )
    };
    let empty = nodes(tree, at).find(bound).is_none();
    nodes(tree, at).filter(bound).chain(empty.then_some(at))
}
/// Local binding of an import specifier. A dynamic `import(..)` and a call of `require` bind no name: their binding is the unknown value.
pub(crate) fn import_name(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    if matches!(
        kind(tree, at),
        Kind::Node(TypeScriptNode::ImportExpression | TypeScriptNode::CallExpression)
    ) {
        return unknown();
    }
    nodes(tree, at)
        .filter(|child| kind(tree, *child) == Kind::Node(TypeScriptNode::Identifier))
        .last()
        .map_or_else(unknown, |name| identifier(tree, symbols, name))
}
/// Module source of an import declaration. A dynamic `import(..)` and a call of `require` name it by their first argument: a string is
/// the module, and anything else is the unknown value, since the module is not known until the program runs. Any other call names none.
pub(crate) fn import_source(
    tree: &Tree<'_>,
    symbols: &mut Symbols,
    at: Position,
) -> Option<Symbol> {
    if kind(tree, at) == Kind::Node(TypeScriptNode::CallExpression) {
        let mut parts = nodes(tree, at);
        let callee = parts.next()?;
        if kind(tree, callee) != Kind::Node(TypeScriptNode::Identifier) || text(tree, callee).0 != REQUIRE {
            return None;
        }
        return Some(
            parts
                .next()
                .and_then(|source| string(tree, symbols, source))
                .unwrap_or_else(unknown),
        );
    }
    if kind(tree, at) == Kind::Node(TypeScriptNode::ImportExpression) {
        return Some(
            nodes(tree, at)
                .next()
                .and_then(|source| string(tree, symbols, source))
                .unwrap_or_else(unknown),
        );
    }
    nodes(tree, at).find_map(|child| string(tree, symbols, child))
}
