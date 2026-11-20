//! The `reaching` service of the model: free functions named by verb that say which definitions of a name can reach a point of
//! the schedule, with no IO and no ports. Every definition that can reach is stated; none is preferred.

use super::types::{Binds, FlowCandidate, FlowReadState, FlowSource, MutationList, ReachModel, RegionRepeat};
use crate::domain::model::definitions::service::definition_of;
use crate::domain::model::frames::service::{lookup, wildcard_frame};
use crate::domain::model::frames::types::{Doubt, FrameCode, FrameModel, NameAbsence};
use crate::domain::model::lexical::constants::MODULE;
use crate::domain::model::declarations::service::{binding_scope, redirected};
use crate::domain::model::lexical::service::{entry, id_of, name_of, node_at, parent_of, scope_at, scope_form, subtree_end_of};
use crate::domain::model::lexical::types::{NamespaceKey, NodeClass, ScopeForm};
use crate::domain::model::names::constants::NO_NAME;
use crate::domain::model::names::service::builtin_of;
use crate::domain::model::names::types::NameCode;
use crate::domain::model::types::BindingAction;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{Count, Position};

/// Forgets the file's candidates and mutations, keeping the buffers.
pub(crate) fn clear(model: &mut ReachModel) {
    model.candidates.clear();
    model.mutations.clear();
    model.mutation_count = Count(0);
}

/// Whether the binding step at `at` gives its name a value: the target of an annotation without a value does not.
fn binds(source: FlowSource<'_, '_, '_>, at: Position) -> Binds {
    if let Some(AnyNodeRef::StmtAnnAssign(statement)) = node_at(source.tree, parent_of(source.tree, at))
        && statement.value.is_none()
    {
        Binds::Nothing
    } else {
        Binds::Value
    }
}

/// The definitions of `name` that can reach `reference` at this point of the schedule, left in the model's candidates, and what
/// they amount to.
pub(crate) fn reach(
    model: &mut ReachModel,
    frames: &FrameModel,
    source: FlowSource<'_, '_, '_>,
    frame: FrameCode,
    reference: Position,
    name: NameCode,
) -> FlowReadState {
    model.candidates.clear();
    let classified = |count, absent, open| {
        if open {
            FlowReadState::Uncertain
        } else if count == 0 {
            FlowReadState::Unbound
        } else if absent {
            FlowReadState::PossiblyUnbound
        } else if count == 1 {
            FlowReadState::Unique
        } else {
            FlowReadState::Alternatives
        }
    };
    let initial = scope_at(source.lexical, reference);
    let mut scope = redirected(source.lexical, initial, name);
    let mut uncertain = false;
    let mut imported = false;
    loop {
        let form = scope_form(entry(&source.lexical.class, scope));
        let visible = scope == initial || form != Some(ScopeForm::Class);
        if visible && wildcard_frame(frames, frame, scope).is_some() {
            imported = true;
        }
        if visible && let Some(found) = lookup(frames, frame, NamespaceKey { scope, name }) {
            for place in usize::from(found.start)..usize::from(found.end) {
                if let Some(binding) = frames.arena.get(place).copied()
                    && binds(source, binding) == Binds::Value
                {
                    model.candidates.push(FlowCandidate { binding, scope });
                }
            }
            uncertain |= found.doubt == Doubt::Uncertain;
            if found.absence == NameAbsence::Present {
                return classified(model.candidates.len(), false, uncertain);
            }
            if matches!(
                form,
                Some(ScopeForm::Function | ScopeForm::Lambda | ScopeForm::Generator | ScopeForm::Comprehension)
            ) {
                return classified(model.candidates.len(), true, uncertain);
            }
        }
        if scope == MODULE {
            break;
        }
        scope = redirected(source.lexical, scope_at(source.lexical, scope), name);
    }
    if uncertain {
        return FlowReadState::Uncertain;
    }
    match builtin_of(source.names, name) {
        Some(since) if model.candidates.is_empty() => FlowReadState::Builtin(since),
        Some(since) => FlowReadState::DeclarationOrBuiltin(since),
        None => classified(model.candidates.len(), true, imported),
    }
}

/// Whether the region `region` can run `reference` again after a later binding inside the region.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "six of the parser's node kinds repeat what is in them; every other runs it once"
)]
fn repeats(source: FlowSource<'_, '_, '_>, reference: Position, region: Position) -> RegionRepeat {
    let tree = source.tree;
    let Some(node) = node_at(source.tree, region) else {
        return RegionRepeat::Once;
    };
    let repeated = match node {
        AnyNodeRef::StmtWhile(_) => true,
        AnyNodeRef::StmtFor(statement) => {
            let mut at = reference;
            loop {
                if at == region || at == MODULE {
                    break false;
                }
                let parent = parent_of(tree, at);
                if parent == region {
                    break id_of(&*statement.iter) != at;
                }
                at = parent;
            }
        }
        AnyNodeRef::ExprListComp(_)
        | AnyNodeRef::ExprSetComp(_)
        | AnyNodeRef::ExprDictComp(_)
        | AnyNodeRef::ExprGenerator(_) => {
            let scope = scope_at(source.lexical, reference);
            region <= scope && scope < subtree_end_of(tree, region)
        }
        _ => false,
    };
    if repeated { RegionRepeat::Repeats } else { RegionRepeat::Once }
}

/// Widens what reaches the read `reference` by what a loop or a handler around it leaves open: a binding later in a loop's body
/// can reach a read earlier in it, and a binding anywhere in a `try` body can reach its handlers.
pub(crate) fn refine(
    model: &mut ReachModel,
    source: FlowSource<'_, '_, '_>,
    reference: Position,
    phase: Position,
    name: NameCode,
    state: FlowReadState,
) -> FlowReadState {
    let tree = source.tree;
    let key = NamespaceKey { scope: binding_scope(source.lexical, tree, reference, name), name };
    let Some(list) = model.mutations.get(&key).copied() else {
        return state;
    };
    let Some(relevant) = model.mutation_lists.get_mut(usize::from(list)).map(std::mem::take) else {
        return state;
    };
    let within = |inner: Position, region: Position| region <= inner && inner < subtree_end_of(tree, region);
    let mut at = reference;
    let mut uncertain = false;
    loop {
        let handler = entry(&source.lexical.class, at) == NodeClass::ExceptHandler;
        let widens = (handler || repeats(source, reference, at) == RegionRepeat::Repeats)
            && !(state == FlowReadState::Unique && model.candidates.iter().all(|candidate| within(candidate.binding, at)));
        if widens {
            let protected = if handler { node_at(source.tree, parent_of(tree, at)) } else { None };
            for mutation in &relevant {
                if mutation.phase != phase {
                    continue;
                }
                let inside = if handler {
                    if let Some(AnyNodeRef::StmtTry(statement)) = protected {
                        statement.body.iter().any(|each| within(mutation.occurrence, id_of(each)))
                    } else {
                        false
                    }
                } else {
                    within(mutation.occurrence, at)
                };
                if !inside {
                    continue;
                }
                uncertain = true;
                if mutation.action == BindingAction::Bind
                    && !model.candidates.iter().any(|candidate| candidate.binding == mutation.occurrence)
                {
                    model.candidates.push(FlowCandidate { binding: mutation.occurrence, scope: key.scope });
                }
            }
        }
        if at == phase || at == MODULE {
            break;
        }
        at = parent_of(tree, at);
    }
    if let Some(slot) = model.mutation_lists.get_mut(usize::from(list)) {
        *slot = relevant;
    }
    if uncertain { FlowReadState::Uncertain } else { state }
}

/// The name and the namespace the binding step at `at` changes.
fn mutated(source: FlowSource<'_, '_, '_>, at: Position) -> Option<NamespaceKey> {
    if let Some(definition) = definition_of(source.definitions, at) {
        return Some(NamespaceKey { scope: definition.owner, name: definition.name });
    }
    let name = name_of(source.lexical, at);
    (name != NO_NAME).then(|| NamespaceKey { scope: binding_scope(source.lexical, source.tree, at, name), name })
}

/// Lists, per name of each namespace, the steps that bind or unbind it.
pub(crate) fn list_mutations(model: &mut ReachModel, source: FlowSource<'_, '_, '_>) {
    for step in &source.schedule.steps {
        if !matches!(step.action, BindingAction::Bind | BindingAction::Delete | BindingAction::Cleanup) {
            continue;
        }
        let Some(key) = mutated(source, step.occurrence) else {
            continue;
        };
        let next = MutationList(u32::try_from(model.mutation_count.0).unwrap_or(u32::MAX));
        let list = *model.mutations.entry(key).or_insert(next);
        if list == next {
            model.mutation_count = Count(model.mutation_count.0.saturating_add(1));
            if let Some(reused) = model.mutation_lists.get_mut(usize::from(list)) {
                reused.clear();
            } else {
                model.mutation_lists.push(Vec::new());
            }
        }
        if let Some(found) = model.mutation_lists.get_mut(usize::from(list)) {
            found.push(*step);
        }
    }
}
