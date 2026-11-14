//! Lexical owners in the TypeScript tree.
use super::types::{Declaredness, Scope};
use crate::domain::names::constants::{OPEN_COMPUTED, OPEN_PARAMETERS, PROPERTY_SEPARATOR};
use crate::domain::names::service::{identifier, name_before, nodes, token, unknown};
use crate::domain::names::types::Spelling;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{
    service::{kind, parent},
    types::Kind,
};
use ryo::languages::{self, Position, Tree};
/// Whether a node introduces a lexical environment.
pub(crate) fn opens(kind: Kind) -> Scope {
    if matches!(
        kind,
        Kind::Node(
            Node::Program
                | Node::FunctionDeclaration
                | Node::FunctionExpression
                | Node::ArrowFunctionExpression
                | Node::ClassDeclaration
                | Node::ClassExpression
                | Node::TSDeclareFunction
                | Node::TSEmptyBodyFunctionExpression
                | Node::TSModuleDeclaration
                | Node::TSModuleBlock
                | Node::BlockStatement
                | Node::CatchClause
                | Node::ForStatement
                | Node::ForInStatement
                | Node::ForOfStatement
                | Node::StaticBlock
        )
    ) {
        Scope::Opens
    } else {
        Scope::Continues
    }
}
/// Nearest enclosing environment; the file owns itself.
pub(crate) fn enclosing(tree: &Tree<'_>, at: Position) -> Position {
    let mut above = parent(tree, at);
    while above != parent(tree, above) && opens(kind(tree, above)) == Scope::Continues {
        above = parent(tree, above);
    }
    above
}
/// The file-qualified owner of an occurrence.
pub(crate) fn owner(tree: &Tree<'_>, at: Position) -> languages::NodeId {
    languages::node_id(tree.file, enclosing(tree, at))
}
/// Whether the function at `at` is declared: a function declaration (`declare function` included), the body of a class method (an
/// abstract one or an overload included), or the body of an object's method, getter or setter (a property with no `:` between its key
/// and its value); every other function is an expression.
pub(crate) fn function_form(tree: &Tree<'_>, at: Position) -> Declaredness {
    match kind(tree, at) {
        Kind::Node(Node::FunctionDeclaration | Node::TSDeclareFunction) => Declaredness::Function,
        Kind::Node(Node::FunctionExpression | Node::TSEmptyBodyFunctionExpression) => {
            let holder = parent(tree, at);
            let method = match kind(tree, holder) {
                Kind::Node(Node::MethodDefinition | Node::TSAbstractMethodDefinition) => true,
                Kind::Node(Node::Property) => {
                    token(tree, holder, Spelling(PROPERTY_SEPARATOR)).is_none()
                }
                Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) | Kind::Pattern(_) => false,
            };
            if !method {
                return Declaredness::Expression;
            }
            let key = nodes(tree, holder).next().filter(|key| {
                token(tree, holder, Spelling(OPEN_COMPUTED)).is_none()
                    && matches!(
                        kind(tree, *key),
                        Kind::Node(Node::Identifier | Node::PrivateIdentifier)
                    )
            });
            Declaredness::Method(key)
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) | Kind::Pattern(_) => {
            Declaredness::Expression
        }
    }
}
/// An explicit callable name: a declaration's own, or a method's key; anonymous functions and arrows remain unknown.
pub(crate) fn function_name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> languages::Symbol {
    match function_form(tree, at) {
        Declaredness::Method(key) => key.map_or_else(unknown, |key| identifier(tree, symbols, key)),
        Declaredness::Expression => unknown(),
        Declaredness::Function => name_before(tree, at, token(tree, at, Spelling(OPEN_PARAMETERS)))
            .map_or_else(unknown, |name| identifier(tree, symbols, name)),
    }
}
