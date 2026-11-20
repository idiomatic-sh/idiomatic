//! The `declarations` service of the model: free functions named by verb, with no IO and no ports. A `global` or a `nonlocal`
//! routes the bindings of a name to another namespace; a deletion leaves the origin of its name unproven after it.

use crate::domain::model::lexical::constants::MODULE;
use crate::domain::model::lexical::service::{entry, name_of, parent_of, scope_at, scope_form, subtree_end_of};
use crate::domain::model::lexical::types::{
    LexicalModel, NameHazard, NamespaceKey, NodeClass, ScopeForm, ScopeRedirect, TypedTree,
};
use crate::domain::model::names::types::NameCode;
use ryo::languages::Position;

/// The scope whose namespace the node `at` binds in, before any `global` or `nonlocal`: a named expression's target binds
/// outside the comprehensions around it.
#[must_use]
pub(crate) fn lexical_scope(model: &LexicalModel, tree: TypedTree<'_, '_>, at: Position) -> Position {
    let mut scope = scope_at(model, at);
    let parent = parent_of(tree, at);
    if entry(&model.class, parent) == NodeClass::NamedExpr && parent.0.saturating_add(1) == at.0 {
        while matches!(
            scope_form(entry(&model.class, scope)),
            Some(ScopeForm::Comprehension | ScopeForm::Generator)
        ) {
            scope = scope_at(model, scope);
        }
    }
    scope
}

/// The nearest function around the scope `scope`; the module when there is none.
fn function_around(model: &LexicalModel, scope: Position) -> Position {
    let mut around = scope_at(model, scope);
    while entry(&model.class, around) != NodeClass::FunctionDef && around != MODULE {
        around = scope_at(model, around);
    }
    around
}

/// Where the scope `scope` says the name `name` is bound instead of in itself, when a `global` or `nonlocal` of it says so.
#[must_use]
pub(crate) fn redirect_of(model: &LexicalModel, scope: Position, name: NameCode) -> Option<ScopeRedirect> {
    if model.declared.is_empty() {
        return None;
    }
    model.declared.get(&NamespaceKey { scope, name }).copied()
}

/// The scope whose namespace holds the name `name` when the scope `scope` binds it.
#[must_use]
pub(crate) fn redirected(model: &LexicalModel, scope: Position, name: NameCode) -> Position {
    match redirect_of(model, scope, name) {
        Some(ScopeRedirect::Global) => MODULE,
        Some(ScopeRedirect::Nonlocal) => function_around(model, scope),
        None => scope,
    }
}

/// The scope whose namespace the node `at` binds the name `name` in.
#[must_use]
pub(crate) fn binding_scope(model: &LexicalModel, tree: TypedTree<'_, '_>, at: Position, name: NameCode) -> Position {
    redirected(model, lexical_scope(model, tree, at), name)
}

/// Records the names the `global` and `nonlocal` statements declare: the first declaration of a name in a scope holds.
fn declare(model: &mut LexicalModel, tree: TypedTree<'_, '_>) {
    for number in 1..u32::try_from(model.nodes.0).unwrap_or(u32::MAX) {
        let at = Position(number);
        let class = entry(&model.class, at);
        let redirect = if class == NodeClass::Global {
            ScopeRedirect::Global
        } else if class == NodeClass::Nonlocal {
            ScopeRedirect::Nonlocal
        } else {
            continue;
        };
        let scope = scope_at(model, at);
        let end = subtree_end_of(tree, at);
        let mut child = Position(number.saturating_add(1));
        while child < end {
            if entry(&model.class, child) == NodeClass::Identifier {
                let key = NamespaceKey { scope, name: name_of(model, child) };
                let _: &mut ScopeRedirect = model.declared.entry(key).or_insert(redirect);
            }
            child = subtree_end_of(tree, child);
        }
    }
}

/// Records the file's declarations and its deletions, once every node's scope and name are known.
pub(crate) fn build(model: &mut LexicalModel, tree: TypedTree<'_, '_>) {
    declare(model, tree);
    for number in 0..model.deleted.len() {
        let Some(at) = model.deleted.get(number).copied() else {
            continue;
        };
        let name = name_of(model, at);
        let scope = binding_scope(model, tree, at, name);
        model.hazards.push(NameHazard { scope, at, name });
    }
    model.hazards.sort_unstable_by_key(|hazard| hazard.at);
}
