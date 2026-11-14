//! The `bindings` service: free functions named by verb that say which declarations the parser's semantic pass resolves a reference
//! to, with no IO and no ports.

use super::constants::{
    ALTERNATIVE, ALTERNATIVE_DECLARATIONS, BUILTIN, BUILTINS, DECLARATIONS, UNBOUND, UNIQUE,
    UNIQUE_DECLARATION,
};
use super::types::{BindingIndex, BoundName, Definition, IndexState};
use crate::domain::names::service::{bindings, declaration_kind, declared, identifier, parameters};
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::enclosing;
use crate::domain::syntax::constants::{REDECLARATION_OF, REFERENCE_DECLARATION, REFERENCE_FLAGS};
use crate::domain::syntax::{
    service::{kind, offset, parameter_kind, parent},
    types::Kind,
};
use ryo::languages::{self, Position, Symbol, Symbols, Tree};
use std::cell::{Ref, RefCell};

/// The names the declaration at `at` binds, each with the `defines` row it gives: the names of a declaration in the scope around it;
/// the name of a function or class expression in the expression itself, the only place it binds; the parameters of a function in the
/// function; and the parameter of a `catch` clause in the clause. Any other node binds none.
pub(crate) fn definitions<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Definition> + 'a {
    let declares = DECLARATIONS.contains(&kind(tree, at));
    let owner = if matches!(
        kind(tree, at),
        Kind::Node(Node::FunctionExpression | Node::ClassExpression | Node::CatchClause)
    ) {
        at
    } else {
        enclosing(tree, at)
    };
    let declaration = declaration_kind(tree, at);
    // The parameters of a function bind in it; a signature's (`(x: T) => U`, an interface's method) bind nothing a body reads.
    let callable = matches!(
        kind(tree, at),
        Kind::Node(
            Node::FunctionDeclaration
                | Node::FunctionExpression
                | Node::ArrowFunctionExpression
                | Node::TSDeclareFunction
                | Node::TSEmptyBodyFunctionExpression
        )
    );
    let parameters = callable
        .then(|| parameters(tree, at))
        .into_iter()
        .flatten()
        .flat_map(move |parameter| {
            bindings(tree, parameter).map(move |binding| (parameter, binding))
        })
        // A `this` parameter types the receiver and declares no name, with or without its annotation.
        .filter(move |(parameter, _)| {
            parameter_kind(tree, *parameter) != languages::ParameterKind::Receiver
        })
        .map(move |(parameter, binding)| Definition {
            id: parameter,
            binding,
            owner: at,
            kind: languages::DeclarationKind::Parameter,
        });
    // An arrow and a method's body bind no name of their own, only their parameters.
    let named = declares
        && !matches!(
            kind(tree, at),
            Kind::Node(Node::ArrowFunctionExpression | Node::TSEmptyBodyFunctionExpression)
        );
    named
        .then(|| declared(tree, at))
        .into_iter()
        .flatten()
        .map(move |binding| Definition {
            id: at,
            binding,
            owner,
            kind: declaration,
        })
        .chain(parameters)
}
/// Empties `index`, keeping its memory, for the next file.
pub(crate) fn clear_bindings(index: &mut BindingIndex) {
    index.names.clear();
    index.state = IndexState::Pending;
}

/// The names the declarations of the file bind, worked out on the first lookup of the file.
pub(crate) fn indexed<'m>(
    tree: &Tree<'_>,
    cell: &'m RefCell<BindingIndex>,
) -> Ref<'m, BindingIndex> {
    if cell.borrow().state == IndexState::Pending {
        let mut index = cell.borrow_mut();
        for at in languages::positions(tree.elements) {
            for definition in definitions(tree, at) {
                index.names.push(BoundName {
                    start: languages::start_at(tree.elements, definition.binding),
                    at: definition.id,
                    redeclares: offset(tree, definition.binding, REDECLARATION_OF),
                });
            }
        }
        index
            .names
            .sort_by_key(|name| (name.redeclares.unwrap_or(name.start), name.start));
        index.state = IndexState::Indexed;
    }
    cell.borrow()
}

/// The declarations the reference at `at` resolves to: the first declaration of its name, then each that declares it again. An
/// unresolved reference, or one to a name no declaration of the file gives a `defines` row, has none.
pub(crate) fn candidates<'i>(
    tree: &Tree<'_>,
    index: &'i BindingIndex,
    at: Position,
) -> &'i [BoundName] {
    let Some(first) = offset(tree, at, REFERENCE_DECLARATION) else {
        return &[];
    };
    let key = |name: &BoundName| name.redeclares.unwrap_or(name.start);
    let from = index.names.partition_point(|name| key(name) < first);
    let to = index.names.partition_point(|name| key(name) <= first);
    index.names.get(from..to).unwrap_or_default()
}

/// The declaration a reference at `at` resolves to first: where its name is first declared.
pub(crate) fn declaration(tree: &Tree<'_>, index: &BindingIndex, at: Position) -> Option<Position> {
    let first = offset(tree, at, REFERENCE_DECLARATION)?;
    candidates(tree, index, at)
        .iter()
        .find(|name| name.start == first)
        .map(|name| name.at)
}

/// How the reference at `at` uses its name, as the catalog's context word; an identifier that is no reference has none.
pub(crate) fn context(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Option<Symbol> {
    let word = languages::property_at(tree.elements, at, REFERENCE_FLAGS)?;
    Some(languages::intern_derived(symbols, tree.source, word))
}

/// What the reference at `at` is bound to, as the catalog's state word: one declaration, a name declared more than once, a name the
/// language provides, or nothing.
pub(crate) fn state(
    tree: &Tree<'_>,
    symbols: &mut Symbols,
    index: &BindingIndex,
    at: Position,
) -> Symbol {
    let name = identifier(tree, symbols, at);
    let word = if languages::property_at(tree.elements, at, REFERENCE_DECLARATION).is_some() {
        if candidates(tree, index, at).len() > 1 {
            ALTERNATIVE_DECLARATIONS
        } else {
            UNIQUE_DECLARATION
        }
    } else if languages::text_of(symbols, tree.source, name)
        .is_some_and(|name| BUILTINS.binary_search(&name.0).is_ok())
    {
        BUILTIN
    } else {
        UNBOUND
    };
    languages::intern_derived(symbols, tree.source, languages::NameText(word))
}

/// The execution boundary the element at `at` runs in: the nearest function around it, else the file.
pub(crate) fn phase(tree: &Tree<'_>, at: Position) -> Position {
    let mut above = parent(tree, at);
    while above != parent(tree, above)
        && !matches!(
            kind(tree, above),
            Kind::Node(
                Node::FunctionDeclaration
                    | Node::FunctionExpression
                    | Node::ArrowFunctionExpression
                    | Node::TSDeclareFunction
                    | Node::TSEmptyBodyFunctionExpression
            )
        )
    {
        above = parent(tree, above);
    }
    above
}

/// How sure a `binding_use` row of one of `candidates` is: the one declaration of its name, or one of several.
pub(crate) fn certainty(
    tree: &Tree<'_>,
    symbols: &mut Symbols,
    candidates: &[BoundName],
) -> Symbol {
    let word = if candidates.len() > 1 {
        ALTERNATIVE
    } else {
        UNIQUE
    };
    languages::intern_derived(symbols, tree.source, languages::NameText(word))
}
