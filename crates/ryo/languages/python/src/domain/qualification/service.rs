//! Lexical import and builtin origins. Dynamic values are unknown, never guessed from spelling.
use super::constants::{
    ADDED_BUILTINS, AMBIGUOUS, BUILTIN, BUILTIN_MODULE, BUILTIN_NAMES, LOCAL, NOT_APPLICABLE,
    PYTHON_THREE, UNRESOLVED,
};
use super::types::{Certainty, Qualified, QualifiedOrigin, Timing, Version};
use crate::domain::{
    bindings::service::resolve,
    definitions::{constants::IMPORT, service::definition},
    extras::types::Extra,
    imports::service::{imported, push_module},
    model::types::{
        Declarations, QualificationHazard, QualificationHazardKind, QualificationIndex,
        QualificationState,
    },
    names::{
        constants::DOT,
        service::{push_identifier, unknown},
        types::{Imported, Unresolved},
    },
    nodes::types::{Category, Node},
    scopes::{
        service::{binding_scope, context, enclosing, nth},
        types::{CategoryIndex, Context},
    },
    syntax::{
        service::{child, identifier_text, kind, name_of, name_token, nodes, parent},
        types::Kind,
    },
};
use ryo::languages::{self, NameText, Position, Symbol, Tree};
use std::cell::Ref;

/// Invalidate events after every parse while retaining their allocation.
pub(crate) fn clear(index: &mut QualificationIndex) {
    index.hazards.clear();
    index.state = QualificationState::Fresh;
}
/// Read provenance only when the entire syntax tree is available.
pub(crate) fn complete<'a>(
    tree: &Tree<'_>,
    model: &'a Declarations,
) -> Option<Ref<'a, QualificationIndex>> {
    if languages::has_incomplete_syntax(tree.elements) {
        return None;
    }
    Some(prepare(tree, model))
}

/// Index wildcard imports and deletion only if provenance is requested.
pub(crate) fn prepare<'a>(tree: &Tree<'_>, model: &'a Declarations) -> Ref<'a, QualificationIndex> {
    if model.qualification.borrow().state == QualificationState::Fresh {
        let mut index = model.qualification.borrow_mut();
        index.hazards.clear();
        for at in languages::positions(tree.elements) {
            let event = match kind(tree, at) {
                Kind::Node(Node::Alias) if imported(tree, at) == Imported::Everything => {
                    Some(QualificationHazard {
                        scope: enclosing(tree, at),
                        position: at,
                        kind: QualificationHazardKind::Wildcard,
                    })
                }
                Kind::Node(Node::Name) if context(tree, at) == Context::Del => name_of(tree, at)
                    .map(|name| QualificationHazard {
                        scope: binding_scope(tree, model, at, name),
                        position: at,
                        kind: QualificationHazardKind::Delete(name),
                    }),
                Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => None,
            };
            if let Some(event) = event {
                index.hazards.push(event);
            }
        }
        index.state = QualificationState::Ready;
    }
    model.qualification.borrow()
}
/// The leftmost name of a dotted expression. Calls and subscripts do not acquire invented origins.
fn root(tree: &Tree<'_>, mut at: Position) -> Option<Position> {
    loop {
        match kind(tree, at) {
            Kind::Node(Node::Name) => return Some(at),
            Kind::Node(Node::Attribute) => {
                at = nth(tree, at, Category::Expression, CategoryIndex(0))?;
            }
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => return None,
        }
    }
}
/// Whether lexical resolution has no named binding after namespace events.
/// A wildcard does not erase an explicit lexical binding or builtin fallback: the
/// relation describes static binding provenance, not which runtime object wins.
fn uncertain(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    name: Symbol,
    binding: Option<Position>,
    index: &QualificationIndex,
) -> Certainty {
    let binding_owner = binding
        .and_then(|binding_position| definition(tree, model, binding_position))
        .map(|found| found.owner);
    let initial = enclosing(tree, at);
    let mut scope = initial;
    let mut timing = Timing::Immediate;
    loop {
        if index.hazards.iter().any(|event| {
            binding.is_none()
                && builtin(tree, at).is_none()
                && event.scope == scope
                && (scope == initial || kind(tree, scope) != Kind::Node(Node::ClassDef))
                && (timing == Timing::Deferred || event.position < at)
                && binding.is_none_or(|bound| event.position > bound)
                && matches!(event.kind, QualificationHazardKind::Wildcard)
        }) || index.hazards.iter().any(|event| {
            event.scope == scope
                && (scope == initial || kind(tree, scope) != Kind::Node(Node::ClassDef))
                && (timing == Timing::Deferred || event.position < at)
                && binding.is_none_or(|bound| event.position > bound)
                && matches!(event.kind, QualificationHazardKind::Delete(deleted) if deleted == name)
        }) {
            return Certainty::Uncertain;
        }
        if binding_owner == Some(scope) || scope == Position(0) {
            return Certainty::Known;
        }
        if matches!(
            kind(tree, scope),
            Kind::Node(
                Node::FunctionDef | Node::AsyncFunctionDef | Node::Lambda | Node::GeneratorExp
            )
        ) {
            timing = Timing::Deferred;
        }
        scope = enclosing(tree, scope);
    }
}
/// The version introducing a builtin, independent of the consumer's target version.
pub(crate) fn builtin(tree: &Tree<'_>, at: Position) -> Option<Version> {
    builtin_named(NameText(
        identifier_text(tree, parent(tree, name_token(tree, at)?)).0,
    ))
}

/// Availability from the same canonical inventory for source reads and occurrence-free catalogs.
pub(crate) fn builtin_named(name: NameText<'_>) -> Option<Version> {
    BUILTIN_NAMES
        .iter()
        .any(|entry| entry.0 == name.0)
        .then(|| {
            ADDED_BUILTINS
                .iter()
                .find(|entry| entry.name.0 == name.0)
                .map_or(PYTHON_THREE, |entry| entry.since)
        })
}
/// The lexical origin selected by the same shadowing rules as `resolves`.
fn origin(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    index: &QualificationIndex,
) -> QualifiedOrigin {
    let Some(name) = name_of(tree, at) else {
        return QualifiedOrigin::Unresolved;
    };
    let binding = resolve(tree, model, at, name);
    if uncertain(tree, model, at, name, binding, index) == Certainty::Uncertain {
        return QualifiedOrigin::Ambiguous;
    }
    match binding {
        Some(binding) if kind(tree, binding) == Kind::Node(Node::Alias) => {
            QualifiedOrigin::Import(binding)
        }
        Some(_) => QualifiedOrigin::Local,
        None if builtin(tree, at).is_some() => QualifiedOrigin::Builtin,
        None => QualifiedOrigin::Unresolved,
    }
}
/// Append the name an import makes available at its binding, not necessarily its full imported path.
fn append_import(tree: &Tree<'_>, symbols: &mut languages::Symbols, alias: Position) {
    let statement = parent(tree, alias);
    let Some(identifier) = child(tree, alias, Kind::Extra(Extra::Identifier)) else {
        return;
    };
    if kind(tree, statement) == Kind::Node(Node::ImportFrom) {
        push_module(tree, symbols, statement);
        if child(tree, statement, Kind::Extra(Extra::Identifier)).is_some() {
            languages::push_text(symbols, NameText(DOT));
        }
        push_identifier(tree, symbols, identifier);
    } else if nodes(tree, alias)
        .filter(|each| kind(tree, *each) == Kind::Extra(Extra::Identifier))
        .nth(1)
        .is_some()
    {
        push_identifier(tree, symbols, identifier);
    } else {
        let Some(token) = name_token(tree, identifier) else {
            return;
        };
        languages::push_text(
            symbols,
            NameText(
                identifier_text(tree, parent(tree, token))
                    .0
                    .split('.')
                    .next()
                    .unwrap_or_default(),
            ),
        );
    }
}
/// Append attributes from inner to outer while retaining exactly the declared root origin.
fn append_attributes(tree: &Tree<'_>, symbols: &mut languages::Symbols, at: Position) {
    if kind(tree, at) != Kind::Node(Node::Attribute) {
        return;
    }
    if let Some(value) = nth(tree, at, Category::Expression, CategoryIndex(0)) {
        append_attributes(tree, symbols, value);
    }
    if let Some(identifier) = child(tree, at, Kind::Extra(Extra::Identifier)) {
        languages::push_text(symbols, NameText(DOT));
        push_identifier(tree, symbols, identifier);
    }
}
/// Resolve one load expression. Unknown cells are ordinary complete answers, not a missing relation.
pub(crate) fn qualify(
    tree: &Tree<'_>,
    model: &Declarations,
    index: &QualificationIndex,
    at: Position,
    symbols: &mut languages::Symbols,
) -> Qualified {
    let base = root(tree, at);
    let origin = base.map_or(QualifiedOrigin::Unresolved, |base| {
        origin(tree, model, base, index)
    });
    let label = match origin {
        QualifiedOrigin::Import(_) => IMPORT,
        QualifiedOrigin::Builtin => BUILTIN,
        QualifiedOrigin::Local => LOCAL,
        QualifiedOrigin::Unresolved => UNRESOLVED,
        QualifiedOrigin::Ambiguous => AMBIGUOUS,
    };
    let provenance = languages::intern_derived(symbols, tree.source, NameText(label));
    let since = if origin == QualifiedOrigin::Builtin {
        base.and_then(|base| builtin(tree, base))
            .unwrap_or(NOT_APPLICABLE)
    } else {
        NOT_APPLICABLE
    };
    let path = if matches!(
        origin,
        QualifiedOrigin::Import(_) | QualifiedOrigin::Builtin
    ) {
        let begin = languages::begin_name(symbols);
        match origin {
            QualifiedOrigin::Import(alias) => append_import(tree, symbols, alias),
            QualifiedOrigin::Builtin => {
                languages::push_text(symbols, NameText(BUILTIN_MODULE));
                languages::push_text(symbols, NameText(DOT));
                if let Some(token) = base.and_then(|base| name_token(tree, base)) {
                    languages::push_text(
                        symbols,
                        NameText(
                            identifier_text(tree, parent(tree, token))
                                .0
                                .split('.')
                                .next()
                                .unwrap_or_default(),
                        ),
                    );
                }
            }
            QualifiedOrigin::Local | QualifiedOrigin::Unresolved | QualifiedOrigin::Ambiguous => {}
        }
        append_attributes(tree, symbols, at);
        languages::end_name(symbols, tree.source, begin)
    } else {
        unknown(Unresolved::QualifiedOrigin)
    };
    Qualified {
        path,
        provenance,
        since,
    }
}
