//! Sparse static environment operations. Joins preserve both real alternatives and possible absence.
use crate::domain::model::types::{
    BindingAbsence, BindingAlternatives, BindingEnvironments, BindingFrame, BindingFrameId,
    BindingFrameKey, BindingKey, BindingKnowledge,
};
use ryo::languages::Position;

/// Reset source-dependent entries while retaining arena, map, and frame capacity.
pub(crate) fn clear(state: &mut BindingEnvironments) {
    state.active = 0;
    state.slots.clear();
    state.candidates.clear();
    state.union.clear();
    state.keys.clear();
    for frame in &mut state.frames {
        frame.touched.clear();
        frame.hazards.clear();
    }
}

/// Open an environment inheriting a completed phase or a branch's entry state.
pub(crate) fn open(
    state: &mut BindingEnvironments,
    parent: Option<BindingFrameId>,
    phase: Position,
) -> BindingFrameId {
    let id = BindingFrameId(state.active);
    state.active += 1;
    if let Some(frame) = state.frames.get_mut(id.0) {
        frame.parent = parent;
        frame.phase = phase;
        frame.touched.clear();
        frame.hazards.clear();
    } else {
        state.frames.push(BindingFrame {
            parent,
            phase,
            touched: Vec::new(),
            hazards: Vec::new(),
        });
    }
    id
}

/// Find a namespace key through sparse overlays. Absence of a slot is not an unknown binding.
pub(crate) fn lookup(
    state: &BindingEnvironments,
    mut frame: BindingFrameId,
    key: BindingKey,
) -> Option<&BindingAlternatives> {
    loop {
        if let Some(found) = state.slots.get(&BindingFrameKey { frame, key }) {
            return Some(found);
        }
        frame = state.frames.get(frame.0)?.parent?;
    }
}

/// The real declaration candidates carried by an established state.
pub(crate) fn candidates<'a>(
    state: &'a BindingEnvironments,
    alternatives: &BindingAlternatives,
) -> &'a [Position] {
    state
        .candidates
        .get(alternatives.candidates.clone())
        .unwrap_or_default()
}

/// Replace one sparse entry, retaining the mutation key only once per frame.
fn store(
    state: &mut BindingEnvironments,
    frame: BindingFrameId,
    key: BindingKey,
    value: BindingAlternatives,
) {
    if state
        .slots
        .insert(BindingFrameKey { frame, key }, value)
        .is_none()
        && let Some(owner) = state.frames.get_mut(frame.0)
    {
        owner.touched.push(key);
    }
}

/// A new declaration has one concrete source identity in the current environment.
pub(crate) fn assign(
    state: &mut BindingEnvironments,
    frame: BindingFrameId,
    key: BindingKey,
    definition: Position,
) {
    let start = state.candidates.len();
    state.candidates.push(definition);
    store(
        state,
        frame,
        key,
        BindingAlternatives {
            candidates: start..state.candidates.len(),
            absence: BindingAbsence::Excluded,
            knowledge: BindingKnowledge::Known,
        },
    );
}

/// Deletion is a supported empty state; it is not an unsupported-capability marker.
pub(crate) fn delete(state: &mut BindingEnvironments, frame: BindingFrameId, key: BindingKey) {
    let end = state.candidates.len();
    store(
        state,
        frame,
        key,
        BindingAlternatives {
            candidates: end..end,
            absence: BindingAbsence::Possible,
            knowledge: BindingKnowledge::Known,
        },
    );
}

/// Preserve a known uncertainty without inventing a binding candidate.
pub(crate) fn uncertain(state: &mut BindingEnvironments, frame: BindingFrameId, key: BindingKey) {
    let value = lookup(state, frame, key)
        .cloned()
        .unwrap_or(BindingAlternatives {
            candidates: state.candidates.len()..state.candidates.len(),
            absence: BindingAbsence::Possible,
            knowledge: BindingKnowledge::Known,
        });
    store(
        state,
        frame,
        key,
        BindingAlternatives {
            knowledge: BindingKnowledge::Uncertain,
            ..value
        },
    );
}

/// Merge mutually exclusive alternatives. Include the entry frame explicitly for a missing
/// else arm or a zero-iteration loop path. A deleted branch and two bound branches differ.
pub(crate) fn join(
    state: &mut BindingEnvironments,
    target: BindingFrameId,
    arms: &[BindingFrameId],
) {
    // Namespace hazards are monotonically conservative across alternatives. Copy only
    // newly introduced hazard identities, never the complete inherited environment.
    for arm in arms {
        let count = state
            .frames
            .get(arm.0)
            .map_or(0, |frame| frame.hazards.len());
        for offset in 0..count {
            if let Some(scope) = state
                .frames
                .get(arm.0)
                .and_then(|frame| frame.hazards.get(offset))
                .copied()
            {
                wildcard(state, target, scope);
            }
        }
    }
    state.keys.clear();
    for arm in arms {
        if *arm != target
            && let Some(frame) = state.frames.get(arm.0)
        {
            state.keys.extend_from_slice(&frame.touched);
        }
    }
    state.keys.sort_unstable();
    state.keys.dedup();
    for index in 0..state.keys.len() {
        let Some(key) = state.keys.get(index).copied() else {
            continue;
        };
        state.union.clear();
        let mut absence = BindingAbsence::Excluded;
        let mut knowledge = BindingKnowledge::Known;
        for arm in arms {
            if let Some(value) = lookup(state, *arm, key).cloned() {
                let values = state.candidates.get(value.candidates).unwrap_or_default();
                state.union.extend_from_slice(values);
                if value.absence == BindingAbsence::Possible {
                    absence = BindingAbsence::Possible;
                }
                if value.knowledge == BindingKnowledge::Uncertain {
                    knowledge = BindingKnowledge::Uncertain;
                }
            } else {
                absence = BindingAbsence::Possible;
            }
        }
        state.union.sort_unstable();
        state.union.dedup();
        let start = state.candidates.len();
        state.candidates.extend_from_slice(&state.union);
        store(
            state,
            target,
            key,
            BindingAlternatives {
                candidates: start..state.candidates.len(),
                absence,
                knowledge,
            },
        );
    }
}

/// Record a namespace-wide import effect without inventing an arbitrary name or binding.
pub(crate) fn wildcard(state: &mut BindingEnvironments, frame: BindingFrameId, scope: Position) {
    if let Some(frame) = state.frames.get_mut(frame.0)
        && !frame.hazards.contains(&scope)
    {
        frame.hazards.push(scope);
    }
}
/// Whether an unresolved name may come from an inherited namespace-wide effect.
pub(crate) fn has_wildcard(
    state: &BindingEnvironments,
    mut frame: BindingFrameId,
    scope: Position,
) -> BindingKnowledge {
    loop {
        let Some(current) = state.frames.get(frame.0) else {
            return BindingKnowledge::Known;
        };
        if current.hazards.contains(&scope) {
            return BindingKnowledge::Uncertain;
        }
        let Some(parent) = current.parent else {
            return BindingKnowledge::Known;
        };
        frame = parent;
    }
}

/// Keep an annotation-only declaration in its namespace without initializing a value.
/// A later annotation never replaces an already established value or deletion.
pub(crate) fn annotate(
    state: &mut BindingEnvironments,
    frame: BindingFrameId,
    key: BindingKey,
    declaration: Position,
) {
    if lookup(state, frame, key).is_some() {
        return;
    }
    assign(state, frame, key, declaration);
    if let Some(value) = state.slots.get_mut(&BindingFrameKey { frame, key }) {
        value.absence = BindingAbsence::Possible;
    }
}
