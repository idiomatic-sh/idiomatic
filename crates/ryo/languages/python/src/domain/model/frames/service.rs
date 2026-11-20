//! The `frames` service of the model: free functions named by verb over the sparse environments of the flow's interpretation,
//! with no IO and no ports. A frame holds only the names it wrote over the frame it was opened from; the arms of alternatives
//! are joined as sets.

use super::types::{
    Doubt, FlowFrame, FrameCode, FrameModel, NameAbsence, ReachPlace, Reaching, SlotKey,
};
use crate::domain::model::lexical::types::NamespaceKey;
use ryo::languages::{Count, Position};

/// Forgets the file's frames, keeping the buffers.
pub(crate) fn clear(model: &mut FrameModel) {
    model.active = Count(0);
    model.slots.clear();
    model.arena.clear();
}

/// Opens a frame over `parent`.
pub(crate) fn open(model: &mut FrameModel, parent: Option<FrameCode>) -> FrameCode {
    let code = FrameCode(u32::try_from(model.active.0).unwrap_or(u32::MAX));
    model.active = Count(model.active.0.saturating_add(1));
    if let Some(frame) = model.frames.get_mut(usize::from(code)) {
        frame.parent = parent;
        frame.touched.clear();
        frame.hazards.clear();
    } else {
        model.frames.push(FlowFrame {
            parent,
            touched: Vec::new(),
            hazards: Vec::new(),
        });
    }
    code
}

/// What `frame` holds of `key`: its own slot, or the nearest one of the frames it was opened from.
pub(crate) fn lookup(model: &FrameModel, frame: FrameCode, key: NamespaceKey) -> Option<Reaching> {
    let mut current = frame;
    loop {
        if let Some(found) = model.slots.get(&SlotKey {
            frame: current,
            key,
        }) {
            return Some(*found);
        }
        current = model.frames.get(usize::from(current))?.parent?;
    }
}

/// Writes what `frame` holds of `key`.
fn store(model: &mut FrameModel, frame: FrameCode, key: NamespaceKey, value: Reaching) {
    if model.slots.insert(SlotKey { frame, key }, value).is_none()
        && let Some(written) = model.frames.get_mut(usize::from(frame))
    {
        written.touched.push(key);
    }
}

/// The end of the arena.
fn arena_end(model: &FrameModel) -> ReachPlace {
    ReachPlace(u32::try_from(model.arena.len()).unwrap_or(u32::MAX))
}

/// Binds `key` to `definition` alone.
pub(crate) fn assign(
    model: &mut FrameModel,
    frame: FrameCode,
    key: NamespaceKey,
    definition: Position,
    absence: NameAbsence,
) {
    let start = arena_end(model);
    model.arena.push(definition);
    let end = arena_end(model);
    store(
        model,
        frame,
        key,
        Reaching {
            start,
            end,
            absence,
            doubt: Doubt::Certain,
        },
    );
}

/// Unbinds `key`.
pub(crate) fn delete(model: &mut FrameModel, frame: FrameCode, key: NamespaceKey) {
    let end = arena_end(model);
    store(
        model,
        frame,
        key,
        Reaching {
            start: end,
            end,
            absence: NameAbsence::Absent,
            doubt: Doubt::Certain,
        },
    );
}

/// Records that a wildcard import wrote the namespace of `scope` in `frame`.
pub(crate) fn wildcard(model: &mut FrameModel, frame: FrameCode, scope: Position) {
    if let Some(written) = model.frames.get_mut(usize::from(frame))
        && !written.hazards.contains(&scope)
    {
        written.hazards.push(scope);
    }
}

/// The frame, `frame` or one it was opened from, in which a wildcard import wrote the namespace of `scope`.
pub(crate) fn wildcard_frame(
    model: &FrameModel,
    frame: FrameCode,
    scope: Position,
) -> Option<FrameCode> {
    let mut current = frame;
    loop {
        let found = model.frames.get(usize::from(current))?;
        if found.hazards.contains(&scope) {
            return Some(current);
        }
        current = found.parent?;
    }
}

/// Joins the arms `arms` into `target`: a name is reached by what reaches it at the end of any arm, and may be absent
/// when an arm leaves it so.
pub(crate) fn join(model: &mut FrameModel, target: FrameCode, arms: &[FrameCode]) {
    let mut keys = std::mem::take(&mut model.keys);
    keys.clear();
    for arm in arms.iter().copied() {
        let Some(hazards) = model
            .frames
            .get_mut(usize::from(arm))
            .map(|frame| std::mem::take(&mut frame.hazards))
        else {
            continue;
        };
        for scope in &hazards {
            wildcard(model, target, *scope);
        }
        if let Some(frame) = model.frames.get_mut(usize::from(arm)) {
            frame.hazards = hazards;
            if arm != target {
                keys.extend_from_slice(&frame.touched);
            }
        }
    }
    keys.sort_unstable();
    keys.dedup();
    let mut union = std::mem::take(&mut model.union);
    for key in &keys {
        union.clear();
        let mut absent = false;
        let mut uncertain = false;
        for arm in arms.iter().copied() {
            if let Some(found) = lookup(model, arm, *key) {
                union.extend_from_slice(
                    model
                        .arena
                        .get(usize::from(found.start)..usize::from(found.end))
                        .unwrap_or_default(),
                );
                absent |= found.absence == NameAbsence::Absent;
                uncertain |= found.doubt == Doubt::Uncertain;
            } else {
                absent = true;
            }
        }
        union.sort_unstable();
        union.dedup();
        let start = arena_end(model);
        model.arena.extend_from_slice(&union);
        let end = arena_end(model);
        store(
            model,
            target,
            *key,
            Reaching {
                start,
                end,
                absence: if absent {
                    NameAbsence::Absent
                } else {
                    NameAbsence::Present
                },
                doubt: if uncertain {
                    Doubt::Uncertain
                } else {
                    Doubt::Certain
                },
            },
        );
    }
    model.union = union;
    model.keys = keys;
}
