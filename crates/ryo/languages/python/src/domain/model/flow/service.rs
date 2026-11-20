//! The `flow` service of the model: free functions named by verb, with no IO and no ports. The schedule is interpreted once:
//! each step that is about a name becomes an event, a read with the definitions that can reach it, or a binding with what it
//! shadows. It states what the language does; it selects nothing.

use super::types::{
    FlowContext, FlowEvent, FlowEventKind, FlowModel, FlowRead, FlowShadow, FlowState, FlowUse, OpenBranch, ShadowForm, UseForm,
};
use crate::domain::model::definitions::service::definition_of;
use crate::domain::model::frames::service::{self as frames, assign, delete, join, lookup, open, wildcard};
use crate::domain::model::frames::types::{FrameCode, NameAbsence};
use crate::domain::model::lexical::constants::MODULE;
use crate::domain::model::declarations::service::{binding_scope, lexical_scope};
use crate::domain::model::lexical::service::{entry, name_of, scope_at};
use crate::domain::model::lexical::types::{AnnotationMark, NamespaceKey, NodeClass};
use crate::domain::model::names::constants::{NO_NAME, UNKNOWN_NAME};
use crate::domain::model::reaching::service::{self as reaching, list_mutations, reach, refine};
use crate::domain::model::reaching::types::{FlowReadState, FlowSource};
use crate::domain::model::types::{BindingAction, BindingStep};
use ryo::languages::node_table::{parent_of, subtree_end_of};
use ryo::languages::{Count, Integer, Position};

/// What the read at `at` is for: a type when it is inside an annotation of its phase.
fn read_context(source: FlowSource<'_, '_, '_>, at: Position, phase: Position, fallback: FlowContext) -> FlowContext {
    if fallback != FlowContext::Value {
        return fallback;
    }
    let mut above = at;
    loop {
        if entry(&source.lexical.annotation, above) == AnnotationMark::Annotation {
            return FlowContext::Type;
        }
        if above == phase || above == MODULE {
            return fallback;
        }
        above = parent_of(source.tree.table, above);
    }
}

/// Records the read of the step: its state, and every definition that can reach it.
fn read(model: &mut FlowModel, source: FlowSource<'_, '_, '_>, frame: FrameCode, step: BindingStep, fallback: FlowContext) {
    let name = name_of(source.lexical, step.occurrence);
    if name == NO_NAME {
        return;
    }
    let context = read_context(source, step.occurrence, step.phase, fallback);
    let reached = reach(&mut model.reach, &model.frames, source, frame, step.occurrence, name);
    let state = refine(&mut model.reach, source, step.occurrence, step.phase, name, reached);
    let form = if state == FlowReadState::Unique { UseForm::Unique } else { UseForm::Alternative };
    model.reads.push(FlowRead {
        at: step.occurrence,
        name,
        scope: scope_at(source.lexical, step.occurrence),
        phase: step.phase,
        context,
        state,
    });
    for candidate in &model.reach.candidates {
        model.uses.push(FlowUse {
            at: step.occurrence,
            binding: candidate.binding,
            scope: candidate.scope,
            phase: step.phase,
            context,
            form,
        });
    }
}

/// Records the binding of the step: what it shadows, then the name's new state in its namespace.
fn bind(model: &mut FlowModel, source: FlowSource<'_, '_, '_>, frame: FrameCode, step: BindingStep) {
    let Some(definition) = definition_of(source.definitions, step.occurrence).copied() else {
        return;
    };
    let key = NamespaceKey { scope: definition.owner, name: definition.name };
    let prior = lookup(&model.frames, frame, key);
    if let Some(found) = prior {
        for place in usize::from(found.start)..usize::from(found.end) {
            if let Some(previous) = model.frames.arena.get(place).copied()
                && previous != step.occurrence
            {
                model.shadows.push(FlowShadow {
                    binding: step.occurrence,
                    previous,
                    scope: definition.owner,
                    previous_scope: definition.owner,
                    phase: step.phase,
                    form: ShadowForm::Replacement,
                });
            }
        }
    }
    let _: FlowReadState = reach(&mut model.reach, &model.frames, source, frame, step.occurrence, definition.name);
    for candidate in &model.reach.candidates {
        if candidate.scope != definition.owner {
            model.shadows.push(FlowShadow {
                binding: step.occurrence,
                previous: candidate.binding,
                scope: definition.owner,
                previous_scope: candidate.scope,
                phase: step.phase,
                form: ShadowForm::Lexical,
            });
        }
    }
    if step.action == BindingAction::Bind {
        assign(&mut model.frames, frame, key, step.occurrence, NameAbsence::Present);
    } else if step.action == BindingAction::Annotation && prior.is_none() {
        assign(&mut model.frames, frame, key, step.occurrence, NameAbsence::Absent);
    } else {
        // An annotation of a name the namespace already holds changes nothing.
    }
}

/// Records the events of a `global` or `nonlocal` statement: one per name it declares.
fn declare(model: &mut FlowModel, source: FlowSource<'_, '_, '_>, step: BindingStep, ordinal: Integer, kind: FlowEventKind) {
    let table = source.tree.table;
    let end = subtree_end_of(table, step.occurrence);
    let mut child = Position(step.occurrence.0.saturating_add(1));
    while child < end {
        if entry(&source.lexical.class, child) == NodeClass::Identifier {
            let name = name_of(source.lexical, child);
            model.events.push(FlowEvent {
                at: child,
                name,
                scope: binding_scope(source.lexical, table, child, name),
                phase: step.phase,
                ordinal,
                kind,
                lexical: lexical_scope(source.lexical, table, child),
            });
        }
        child = subtree_end_of(table, child);
    }
}

/// Interprets one step that is about a name: records its event, then what it reads or changes.
fn observe(model: &mut FlowModel, source: FlowSource<'_, '_, '_>, frame: FrameCode, step: BindingStep, ordinal: Integer) {
    let table = source.tree.table;
    let definition = definition_of(source.definitions, step.occurrence).copied();
    let kind = match step.action {
        BindingAction::Read => FlowEventKind::Read,
        BindingAction::Bind if definition.is_some_and(|found| found.name == UNKNOWN_NAME) => FlowEventKind::Wildcard,
        BindingAction::Bind => FlowEventKind::Bind,
        BindingAction::Annotation => FlowEventKind::Annotation,
        BindingAction::Delete => FlowEventKind::Delete,
        BindingAction::Cleanup => FlowEventKind::Cleanup,
        BindingAction::Global => return declare(model, source, step, ordinal, FlowEventKind::Global),
        BindingAction::Nonlocal => return declare(model, source, step, ordinal, FlowEventKind::Nonlocal),
        BindingAction::BeginPhase
        | BindingAction::EndPhase
        | BindingAction::BeginAlternatives
        | BindingAction::NextAlternative
        | BindingAction::EndAlternatives => return,
    };
    let name = definition.map_or_else(|| name_of(source.lexical, step.occurrence), |found| found.name);
    if name == NO_NAME {
        return;
    }
    let scope = if kind == FlowEventKind::Read {
        scope_at(source.lexical, step.occurrence)
    } else {
        definition.map_or_else(|| binding_scope(source.lexical, table, step.occurrence, name), |found| found.owner)
    };
    let written_in_scope = kind == FlowEventKind::Read
        || matches!(
            entry(&source.lexical.class, step.occurrence),
            NodeClass::Parameter | NodeClass::TypeVar | NodeClass::TypeVarTuple | NodeClass::ParamSpec
        );
    let lexical = if written_in_scope { scope } else { lexical_scope(source.lexical, table, step.occurrence) };
    model.events.push(FlowEvent { at: step.occurrence, name, scope, phase: step.phase, ordinal, kind, lexical });
    let key = NamespaceKey { scope, name };
    match kind {
        FlowEventKind::Wildcard => wildcard(&mut model.frames, frame, scope),
        FlowEventKind::Read => read(model, source, frame, step, FlowContext::Value),
        FlowEventKind::Bind | FlowEventKind::Annotation => bind(model, source, frame, step),
        FlowEventKind::Delete => {
            read(model, source, frame, step, FlowContext::Deletion);
            delete(&mut model.frames, frame, key);
        }
        FlowEventKind::Cleanup => delete(&mut model.frames, frame, key),
        FlowEventKind::Global | FlowEventKind::Nonlocal => {}
    }
}

/// Forgets the file, keeping the buffers.
fn clear(model: &mut FlowModel) {
    model.events.clear();
    model.reads.clear();
    model.uses.clear();
    model.shadows.clear();
    model.state = FlowState::Complete;
    frames::clear(&mut model.frames);
    reaching::clear(&mut model.reach);
    model.phase_frames.clear();
    model.branches.clear();
    model.arms.clear();
}

/// Interprets the schedule of one file. A schedule whose phases or alternatives do not pair up leaves the flow incomplete.
pub(crate) fn build(model: &mut FlowModel, source: FlowSource<'_, '_, '_>) {
    clear(model);
    list_mutations(&mut model.reach, source);
    model.state = FlowState::Incomplete;
    let mut frame: Option<FrameCode> = None;
    let mut later = 0_usize;
    for (cursor, step) in source.schedule.steps.iter().enumerate() {
        match step.action {
            BindingAction::BeginPhase => {
                let parent = if cursor == 0 {
                    None
                } else {
                    let task = source.schedule.phases.get(later);
                    later = later.saturating_add(1);
                    let Some(found) = task.and_then(|phase| model.phase_frames.get(&phase.parent).copied()) else {
                        return;
                    };
                    Some(found)
                };
                frame = Some(open(&mut model.frames, parent));
            }
            BindingAction::EndPhase => {
                if !model.branches.is_empty() {
                    return;
                }
                if let Some(finished) = frame.take() {
                    let _: Option<FrameCode> = model.phase_frames.insert(step.occurrence, finished);
                }
            }
            BindingAction::BeginAlternatives => {
                let Some(current) = frame else {
                    return;
                };
                model.branches.push(OpenBranch { target: current, arms: Count(model.arms.len()) });
                frame = Some(open(&mut model.frames, Some(current)));
            }
            BindingAction::NextAlternative => {
                let (Some(branch), Some(arm)) = (model.branches.last().copied(), frame) else {
                    return;
                };
                model.arms.push(arm);
                frame = Some(open(&mut model.frames, Some(branch.target)));
            }
            BindingAction::EndAlternatives => {
                let (Some(branch), Some(arm)) = (model.branches.pop(), frame) else {
                    return;
                };
                model.arms.push(arm);
                join(&mut model.frames, branch.target, model.arms.get(branch.arms.0..).unwrap_or_default());
                model.arms.truncate(branch.arms.0);
                frame = Some(branch.target);
            }
            BindingAction::Read
            | BindingAction::Bind
            | BindingAction::Annotation
            | BindingAction::Delete
            | BindingAction::Cleanup
            | BindingAction::Global
            | BindingAction::Nonlocal => {
                let Some(current) = frame else {
                    return;
                };
                let ordinal = Integer(i64::try_from(cursor).unwrap_or(i64::MAX));
                observe(model, source, current, *step, ordinal);
            }
        }
    }
    model.state = FlowState::Complete;
}
