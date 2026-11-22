//! The `declarations` service of the model: free functions named by verb, with no IO and no ports. A `global` or a `nonlocal`
//! routes the bindings of a name to another namespace; a deletion leaves the origin of its name unproven after it.

use crate::domain::model::lexical::constants::MODULE;
use crate::domain::model::lexical::service::{entry, name_of, parent_of, scope_at, scope_form, subtree_end_of};
use crate::domain::model::lexical::types::{
    LexicalModel, NameHazard, NamespaceKey, NodeClass, ScopeForm, ScopeRedirect, TypedTree,
};
use crate::domain::model::definitions::service::definitions_of;
use crate::domain::model::definitions::types::{DefinitionModel, NameDefinition};
use crate::domain::model::names::constants::NO_NAME;
use crate::domain::model::names::types::NameCode;
use ryo::languages::{DeclarationKind, Position};

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
        Some(ScopeRedirect::Nonlocal) => model
            .targets
            .get(&NamespaceKey { scope, name })
            .copied()
            .unwrap_or_else(|| function_around(model, scope)),
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

/// The scope whose block binds the definition `definition` before any `global` or `nonlocal`: a parameter's and a type
/// parameter's is their owner.
fn written_in(model: &LexicalModel, tree: TypedTree<'_, '_>, definition: &NameDefinition) -> Position {
    if matches!(definition.kind, DeclarationKind::Parameter | DeclarationKind::TypeParameter) {
        definition.owner
    } else {
        lexical_scope(model, tree, definition.at)
    }
}

/// Whether the block of the function `function` binds the name `name` itself.
fn binds_itself(
    model: &LexicalModel,
    definitions: &DefinitionModel,
    tree: TypedTree<'_, '_>,
    function: Position,
    name: NameCode,
) -> bool {
    definitions_of(definitions, function, name).iter().any(|code| {
        definitions
            .all
            .get(usize::from(*code))
            .is_some_and(|definition| written_in(model, tree, definition) == function)
    })
}

/// The function the `nonlocal` name `name` of the scope `scope` is bound in: the nearest enclosing function whose block binds
/// the name, passing through a function that declares it `nonlocal` too; none when a function declares it `global` first or
/// none binds it (the parser's error).
fn nonlocal_target(
    model: &LexicalModel,
    definitions: &DefinitionModel,
    tree: TypedTree<'_, '_>,
    scope: Position,
    name: NameCode,
) -> Option<Position> {
    let mut around = function_around(model, scope);
    while around != MODULE {
        match redirect_of(model, around, name) {
            Some(ScopeRedirect::Global) => return None,
            Some(ScopeRedirect::Nonlocal) => {}
            None if binds_itself(model, definitions, tree, around, name) => return Some(around),
            None => {}
        }
        around = function_around(model, around);
    }
    None
}

/// Routes each `nonlocal` name to the function that binds it, once the definitions are known, and re-routes the deletions
/// of the names it moves. Answers whether any name moved off the nearest function around its scope: the definitions are then
/// to be built again.
pub(crate) fn routed(model: &mut LexicalModel, definitions: &DefinitionModel, tree: TypedTree<'_, '_>) -> bool {
    let mut moved = Vec::new();
    for (key, redirect) in &model.declared {
        if *redirect != ScopeRedirect::Nonlocal {
            continue;
        }
        if let Some(target) = nonlocal_target(model, definitions, tree, key.scope, key.name)
            && target != function_around(model, key.scope)
        {
            moved.push((*key, target));
        }
    }
    if moved.is_empty() {
        return false;
    }
    model.targets.extend(moved);
    for number in 0..model.hazards.len() {
        let Some(hazard) = model.hazards.get(number).copied() else {
            continue;
        };
        if hazard.name != NO_NAME {
            let scope = binding_scope(model, tree, hazard.at, hazard.name);
            if let Some(slot) = model.hazards.get_mut(number) {
                slot.scope = scope;
            }
        }
    }
    true
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
