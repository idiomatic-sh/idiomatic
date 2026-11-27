//! The types of the model's `frames` part: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::model::lexical::types::NamespaceKey;
use crate::domain::model::names::types::KeyHasher;
use ryo::languages::{Count, Position};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

/// A place in the arena of reaching definitions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ReachPlace(pub u32);

impl From<ReachPlace> for usize {
    fn from(place: ReachPlace) -> Self {
        Self::try_from(place.0).unwrap_or(Self::MAX)
    }
}

/// Whether a name may be absent from a namespace at a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameAbsence {
    /// Every path bound it.
    Present,
    /// Some path left it unbound.
    Absent,
}

/// Whether what reaches a point is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Doubt {
    /// It is known.
    Certain,
    /// It is open.
    Uncertain,
}

/// The definitions of one name that can reach a point, as a range of the arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Reaching {
    /// The first.
    pub start: ReachPlace,
    /// One past the last.
    pub end: ReachPlace,
    /// Whether the name may also be absent.
    pub(crate) absence: NameAbsence,
    /// Whether the answer is open.
    pub(crate) doubt: Doubt,
}

/// One environment of the interpretation: a phase, or an arm of alternatives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FrameCode(pub u32);

impl From<FrameCode> for usize {
    fn from(code: FrameCode) -> Self {
        Self::try_from(code.0).unwrap_or(Self::MAX)
    }
}

/// A name of a namespace as one frame holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SlotKey {
    /// The frame.
    pub(crate) frame: FrameCode,
    /// The namespace and the name.
    pub(crate) key: NamespaceKey,
}

/// One frame: what it changed over the frame it was opened from.
#[derive(Clone, Debug, Default)]
pub(crate) struct FlowFrame {
    /// The frame it was opened from; none for the module's.
    pub(crate) parent: Option<FrameCode>,
    /// The names it wrote, each once.
    pub(crate) touched: Vec<NamespaceKey>,
    /// The namespaces a wildcard import wrote in it.
    pub(crate) hazards: Vec<Position>,
}

/// The frames of one file's interpretation, cleared between files.
#[derive(Clone, Debug, Default)]
pub(crate) struct FrameModel {
    /// The frames, reused between files.
    pub(crate) frames: Vec<FlowFrame>,
    /// How many frames this file has opened.
    pub(crate) active: Count,
    /// What each frame holds of each name it wrote.
    pub(crate) slots: HashMap<SlotKey, Reaching, BuildHasherDefault<KeyHasher>>,
    /// The reaching definitions the slots point into.
    pub(crate) arena: Vec<Position>,
    /// The definitions of one name being joined.
    pub(crate) union: Vec<Position>,
    /// The names being joined.
    pub keys: Vec<NamespaceKey>,
}
