//! The `resolution` service of the model: free functions named by verb, with no IO and no ports.
//!
//! `resolved_at` and what reads it (the `resolves` and `qualified-name` relations) give each name that is read ONE definition,
//! by a static rule: the last definition of the name before the read in source order, in the nearest namespace the read can
//! see, or the first one when that namespace is a function's, a lambda's or a comprehension's and none comes before. It is not
//! the set of definitions that can reach the read: `if c: import a as m / else: import b as m / m.f()` answers the second
//! import alone. The reaching definitions are the flow part's (`binding-read`, `binding-use`). The two relations keep this
//! meaning; whether they become the reaching set, with the selection in the rule pack, is the product owner's open decision.

use super::types::{Bound, OriginProof, ResolutionModel};
use crate::domain::model::declarations::service::redirected;
use crate::domain::model::definitions::service::{definition_of, definitions_of, place_of};
use crate::domain::model::definitions::types::DefinitionModel;
use crate::domain::model::lexical::constants::{MODULE, NO_NODE};
use crate::domain::model::lexical::service::{entry, name_of, scope_at, scope_form};
use crate::domain::model::lexical::types::{LexicalModel, NameUse, NodeClass, ScopeForm};
use crate::domain::model::names::constants::NO_NAME;
use crate::domain::model::names::service::builtin_of;
use crate::domain::model::names::types::{NameCode, NameTable};
use ryo::languages::node_table::{parent_of, subtree_end_of};
use ryo::languages::{NodeTable, Position};

/// Whether the definition `binding` is in effect at `occurrence`: an assignment's value is evaluated before its targets are
/// bound, and a definition's own decorators, defaults and bases before its name.
fn bound(
    lexical: &LexicalModel,
    table: &NodeTable,
    binding: Position,
    occurrence: Position,
) -> Bound {
    let mut above = binding;
    loop {
        let class = entry(&lexical.class, above);
        let evaluating = matches!(
            class,
            NodeClass::Assign | NodeClass::AnnAssign | NodeClass::NamedExpr
        ) || (above == binding
            && matches!(class, NodeClass::FunctionDef | NodeClass::ClassDef));
        if evaluating {
            return if occurrence >= subtree_end_of(table, above) || occurrence < above {
                Bound::Yes
            } else {
                Bound::NotYet
            };
        }
        if matches!(
            class,
            NodeClass::Module | NodeClass::FunctionDef | NodeClass::ClassDef | NodeClass::Lambda
        ) || above == MODULE
        {
            return Bound::Yes;
        }
        above = parent_of(table, above);
    }
}

/// The one definition the name `name` read at `at` resolves to, by the static rule this file's header states.
#[must_use]
pub(crate) fn resolve(
    model: &DefinitionModel,
    lexical: &LexicalModel,
    table: &NodeTable,
    at: Position,
    name: NameCode,
) -> Option<Position> {
    let initial = scope_at(lexical, at);
    let mut scope = initial;
    let mut delayed = false;
    loop {
        let lookup = redirected(lexical, scope, name);
        let scope_runs_later = matches!(
            scope_form(entry(&lexical.class, scope)),
            Some(ScopeForm::Function | ScopeForm::Lambda | ScopeForm::Generator)
        );
        delayed |= lookup != scope && scope_runs_later;
        let form = scope_form(entry(&lexical.class, lookup));
        if form != Some(ScopeForm::Class) || scope == initial {
            let range = definitions_of(model, lookup, name);
            let local = matches!(
                form,
                Some(
                    ScopeForm::Function
                        | ScopeForm::Lambda
                        | ScopeForm::Comprehension
                        | ScopeForm::Generator
                )
            );
            let before = if delayed {
                range.len()
            } else {
                range.partition_point(|code| place_of(model, *code) < at)
            };
            let found = range
                .get(..before)
                .unwrap_or_default()
                .iter()
                .rev()
                .find(|code| {
                    delayed
                        || local
                        || bound(lexical, table, place_of(model, **code), at) == Bound::Yes
                });
            if let Some(code) = found {
                return Some(place_of(model, *code));
            }
            if local && let Some(first) = range.first() {
                return Some(place_of(model, *first));
            }
        }
        delayed |= scope_runs_later;
        if lookup == MODULE {
            return None;
        }
        scope = scope_at(lexical, scope);
    }
}

/// The definition the name expression `at` resolves to, when it is read and resolves to one.
#[must_use]
pub(crate) fn resolved_at(model: &ResolutionModel, at: Position) -> Option<Position> {
    model
        .resolved
        .get(usize::from(at))
        .copied()
        .filter(|found| *found != NO_NODE)
}

/// Whether the origin of the name `name` read at `at`, resolved to `binding`, is proven: a wildcard import or a deletion between
/// the definition and the read leaves it unproven.
#[must_use]
pub(crate) fn proof_of(
    model: &DefinitionModel,
    lexical: &LexicalModel,
    names: &NameTable,
    at: Position,
    name: NameCode,
    binding: Option<Position>,
) -> OriginProof {
    if lexical.hazards.is_empty() {
        return OriginProof::Proven;
    }
    let owner =
        binding.and_then(|found| definition_of(model, found).map(|definition| definition.owner));
    let builtin = builtin_of(names, name).is_some();
    let initial = scope_at(lexical, at);
    let mut scope = initial;
    let mut deferred = false;
    loop {
        let form = scope_form(entry(&lexical.class, scope));
        if scope == initial || form != Some(ScopeForm::Class) {
            for hazard in &lexical.hazards {
                if hazard.scope != scope || !(deferred || hazard.at < at) {
                    continue;
                }
                let unproven = if hazard.name == NO_NAME {
                    binding.is_none() && !builtin
                } else {
                    hazard.name == name && binding.is_none_or(|found| hazard.at > found)
                };
                if unproven {
                    return OriginProof::Unproven;
                }
            }
        }
        if owner == Some(scope) || scope == MODULE {
            return OriginProof::Proven;
        }
        if matches!(
            form,
            Some(ScopeForm::Function | ScopeForm::Lambda | ScopeForm::Generator)
        ) {
            deferred = true;
        }
        scope = scope_at(lexical, scope);
    }
}

/// Resolves every name expression of the file that is read.
pub(crate) fn build(
    model: &mut ResolutionModel,
    definitions: &DefinitionModel,
    lexical: &LexicalModel,
    table: &NodeTable,
) {
    model.resolved.clear();
    let count = lexical.class.len();
    model.resolved.resize(count, NO_NODE);
    for number in 0..u32::try_from(count).unwrap_or(u32::MAX) {
        let at = Position(number);
        if entry(&lexical.class, at) == NodeClass::Name
            && entry(&lexical.usage, at) == NameUse::Load
        {
            let found = resolve(definitions, lexical, table, at, name_of(lexical, at));
            if let Some(slot) = model.resolved.get_mut(usize::from(at)) {
                *slot = found.unwrap_or(NO_NODE);
            }
        }
    }
}
