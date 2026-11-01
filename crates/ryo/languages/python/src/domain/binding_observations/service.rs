//! Convert executed schedule events into declaration identities, explicit reads and shadow edges.
use crate::domain::{
    binding_lookup::service::resolve,
    binding_results::types::{
        Event, EventKind, Read, ReadContext, ReadState, Shadow, ShadowRelationship, Usage,
        UseCertainty,
    },
    binding_states::service::{annotate, assign, candidates, delete, lookup, wildcard},
    definitions::service::definition,
    fields::service::field_of,
    imports::service::imported,
    labels::types::Field,
    model::types::{
        BindingAction, BindingAnalysisIndex, BindingFrameId, BindingKey, BindingStep, Declarations,
    },
    names::types::Imported,
    nodes::types::Node,
    scopes::service::{binding_scope, enclosing},
    syntax::{
        service::{kind, name_of, nodes, parent},
        types::Kind,
    },
};
use ryo::languages::{Integer, Position, Tree};

/// Native annotation fields are type reads even when their evaluation phase is eager.
fn read_context(
    tree: &Tree<'_>,
    mut at: Position,
    phase: Position,
    fallback: ReadContext,
) -> ReadContext {
    if fallback != ReadContext::Value {
        return fallback;
    }
    loop {
        if matches!(field_of(tree, at), Field::Annotation | Field::Returns) {
            return ReadContext::Type;
        }
        if at == phase || at == Position(0) {
            return fallback;
        }
        let above = parent(tree, at);
        if above == at {
            return fallback;
        }
        at = above;
    }
}

/// Record a read and only its real declaration edges. Builtin/absence has no fake Node.
fn read(
    tree: &Tree<'_>,
    model: &Declarations,
    frame: BindingFrameId,
    step: BindingStep,
    context: ReadContext,
    index: &mut BindingAnalysisIndex,
) {
    let Some(name) = name_of(tree, step.occurrence) else {
        return;
    };
    let context = read_context(tree, step.occurrence, step.phase, context);
    let state = resolve(
        tree,
        model,
        &index.environments,
        frame,
        step.occurrence,
        name,
        &mut index.candidates,
    );
    let state = crate::domain::binding_uncertainty::service::refine(
        crate::domain::binding_uncertainty::service::Refinement {
            tree,
            model,
            reference: step.occurrence,
            phase: step.phase,
            name,
            state,
            mutations: &index.mutations,
            candidates: &mut index.candidates,
        },
    );
    let certainty = match state {
        ReadState::UniqueDeclaration => UseCertainty::Unique,
        ReadState::AlternativeDeclarations
        | ReadState::PossiblyUnbound
        | ReadState::DeclarationOrBuiltin { .. }
        | ReadState::Uncertain
        | ReadState::Unbound
        | ReadState::Builtin { .. } => UseCertainty::Alternative,
    };
    index.reads.push(Read {
        reference: step.occurrence,
        name,
        scope: enclosing(tree, step.occurrence),
        phase: step.phase,
        context,
        state,
    });
    index
        .uses
        .extend(index.candidates.iter().map(|candidate| Usage {
            reference: step.occurrence,
            binding: candidate.binding,
            scope: candidate.scope,
            phase: step.phase,
            context,
            certainty,
        }));
}

/// Save real declaration shadows independently of whether the declaration initializes a value.
fn bind(
    tree: &Tree<'_>,
    model: &Declarations,
    frame: BindingFrameId,
    step: BindingStep,
    index: &mut BindingAnalysisIndex,
) {
    let Some(declared) = definition(tree, model, step.occurrence) else {
        return;
    };
    if kind(tree, step.occurrence) == Kind::Node(Node::Alias)
        && imported(tree, step.occurrence) == Imported::Everything
    {
        wildcard(&mut index.environments, frame, declared.owner);
        return;
    }
    let key = BindingKey {
        scope: declared.owner,
        name: declared.name,
    };
    if let Some(prior) = lookup(&index.environments, frame, key) {
        index.shadows.extend(
            candidates(&index.environments, prior)
                .iter()
                .filter(|previous| **previous != step.occurrence)
                .map(|previous| Shadow {
                    binding: step.occurrence,
                    previous: *previous,
                    scope: declared.owner,
                    previous_scope: declared.owner,
                    phase: step.phase,
                    relationship: ShadowRelationship::Replacement,
                }),
        );
    }
    // An outer static association is retained independently of same-namespace replacement.
    let _state = resolve(
        tree,
        model,
        &index.environments,
        frame,
        step.occurrence,
        declared.name,
        &mut index.candidates,
    );
    index.shadows.extend(
        index
            .candidates
            .iter()
            .filter(|candidate| candidate.scope != declared.owner)
            .map(|candidate| Shadow {
                binding: step.occurrence,
                previous: candidate.binding,
                scope: declared.owner,
                previous_scope: candidate.scope,
                phase: step.phase,
                relationship: ShadowRelationship::Lexical,
            }),
    );
    match step.action {
        BindingAction::Bind => assign(&mut index.environments, frame, key, step.occurrence),
        BindingAction::Annotation => annotate(&mut index.environments, frame, key, step.occurrence),
        BindingAction::Read
        | BindingAction::Delete
        | BindingAction::Cleanup
        | BindingAction::Global
        | BindingAction::Nonlocal
        | BindingAction::BeginPhase
        | BindingAction::EndPhase
        | BindingAction::BeginAlternatives
        | BindingAction::NextAlternative
        | BindingAction::EndAlternatives => {}
    }
}

/// Record one actual syntax event. Control markers are consumed by the analysis orchestrator.
pub(crate) fn observe(
    tree: &Tree<'_>,
    model: &Declarations,
    frame: BindingFrameId,
    step: BindingStep,
    context: ReadContext,
    ordinal: Integer,
    index: &mut BindingAnalysisIndex,
) {
    let event_kind = if step.action == BindingAction::Bind
        && kind(tree, step.occurrence) == Kind::Node(Node::Alias)
        && imported(tree, step.occurrence) == Imported::Everything
    {
        EventKind::Wildcard
    } else {
        match step.action {
            BindingAction::Read => EventKind::Read,
            BindingAction::Bind => EventKind::Bind,
            BindingAction::Annotation => EventKind::Annotation,
            BindingAction::Delete => EventKind::Delete,
            BindingAction::Cleanup => EventKind::Cleanup,
            BindingAction::Global => EventKind::Global,
            BindingAction::Nonlocal => EventKind::Nonlocal,
            BindingAction::BeginPhase
            | BindingAction::EndPhase
            | BindingAction::BeginAlternatives
            | BindingAction::NextAlternative
            | BindingAction::EndAlternatives => return,
        }
    };
    if matches!(event_kind, EventKind::Global | EventKind::Nonlocal) {
        for occurrence in nodes(tree, step.occurrence) {
            if let Some(name) = name_of(tree, occurrence) {
                index.events.push(Event {
                    occurrence,
                    name,
                    scope: binding_scope(tree, model, occurrence, name),
                    phase: step.phase,
                    ordinal,
                    kind: event_kind,
                });
            }
        }
        return;
    }
    let declaration = definition(tree, model, step.occurrence);
    let Some(name) = declaration
        .map(|value| value.name)
        .or_else(|| name_of(tree, step.occurrence))
    else {
        return;
    };
    let scope = if event_kind == EventKind::Read {
        enclosing(tree, step.occurrence)
    } else {
        declaration.map_or_else(
            || binding_scope(tree, model, step.occurrence, name),
            |value| value.owner,
        )
    };
    index.events.push(Event {
        occurrence: step.occurrence,
        name,
        scope,
        phase: step.phase,
        ordinal,
        kind: event_kind,
    });
    match event_kind {
        EventKind::Wildcard => wildcard(&mut index.environments, frame, scope),
        EventKind::Read => read(tree, model, frame, step, context, index),
        EventKind::Bind | EventKind::Annotation => bind(tree, model, frame, step, index),
        EventKind::Delete => {
            read(tree, model, frame, step, ReadContext::Deletion, index);
            delete(&mut index.environments, frame, BindingKey { scope, name });
        }
        EventKind::Cleanup => delete(&mut index.environments, frame, BindingKey { scope, name }),
        EventKind::Global | EventKind::Nonlocal => {}
    }
}
