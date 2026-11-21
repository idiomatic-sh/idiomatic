//! The types of the model's `flow` part: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::model::frames::types::{FrameCode, FrameModel};
use crate::domain::model::lexical::types::NamespaceKey;
use crate::domain::model::names::types::{KeyHasher, NameCode};
use crate::domain::model::reaching::types::{FlowReadState, ReachModel};
use ryo::languages::{BindingResolutionRow, Count, Integer, Position};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

/// What an event does to a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlowEventKind {
    /// A wildcard import binds names unknown here.
    Wildcard,
    /// The name is read.
    Read,
    /// The name is bound.
    Bind,
    /// The name is annotated without a value.
    Annotation,
    /// The name is deleted.
    Delete,
    /// An `except ... as` name is unbound when its handler ends.
    Cleanup,
    /// The name is declared `global`.
    Global,
    /// The name is declared `nonlocal`.
    Nonlocal,
}

/// One thing the language does with a name, in evaluation order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FlowEvent {
    /// The node.
    pub at: Position,
    /// The name; the unknown name for a wildcard import.
    pub name: NameCode,
    /// The scope whose namespace the event is about: where a read starts, where a binding lands.
    pub scope: Position,
    /// The phase the event runs in.
    pub phase: Position,
    /// The event's place in the schedule. Only its order means anything: the numbers skip the schedule's markers.
    pub ordinal: Integer,
    /// What it does.
    pub kind: FlowEventKind,
    /// The scope the event is written in, before any `global` or `nonlocal`.
    pub lexical: Position,
}

/// What a read is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlowContext {
    /// A value.
    Value,
    /// A type: the read is inside an annotation.
    Type,
    /// A deletion.
    Deletion,
}

/// One read of a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FlowRead {
    /// The name expression.
    pub at: Position,
    /// The name.
    pub name: NameCode,
    /// The scope the read is evaluated in.
    pub scope: Position,
    /// The phase the read runs in.
    pub phase: Position,
    /// What the read is for.
    pub context: FlowContext,
    /// What can reach it.
    pub state: FlowReadState,
}

/// Whether a use is of the one definition that reaches the read, or of one among several.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UseForm {
    /// The only one.
    Unique,
    /// One among several.
    Alternative,
}

/// One definition that can reach a read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FlowUse {
    /// The read.
    pub at: Position,
    /// The definition.
    pub binding: Position,
    /// The scope whose namespace held the definition.
    pub scope: Position,
    /// The phase the read runs in.
    pub phase: Position,
    /// What the read is for.
    pub context: FlowContext,
    /// Whether it is the only one.
    pub form: UseForm,
}

/// How a binding shadows an earlier one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShadowForm {
    /// It hides a definition of an enclosing namespace.
    Lexical,
    /// It replaces a definition of its own namespace.
    Replacement,
}

/// One binding that shadows an earlier definition of the same name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FlowShadow {
    /// The later binding.
    pub binding: Position,
    /// The earlier definition.
    pub previous: Position,
    /// The namespace of the later binding.
    pub scope: Position,
    /// The namespace of the earlier definition.
    pub previous_scope: Position,
    /// The phase the later binding runs in.
    pub phase: Position,
    /// How it shadows.
    pub form: ShadowForm,
}

/// One read or one shadow in a list by key: what the list is ordered by, and its place among the reads or the shadows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KeyPlace<K> {
    /// What the list is ordered by: the name a read reads, the name expression of a read, or the binding of a shadow.
    pub key: K,
    /// The place among the reads, or among the shadows.
    pub at: Count,
}

/// Whether the flow of a file could be interpreted to its end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FlowState {
    /// It could.
    #[default]
    Complete,
    /// It could not: the schedule's phases or alternatives do not pair up.
    Incomplete,
}

/// Alternatives that are open: the frame they join into, and where their arms start among the open arms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OpenBranch {
    /// The frame the arms join into.
    pub target: FrameCode,
    /// Where its arms start.
    pub arms: Count,
}

/// The flow part of the model, cleared between files.
#[derive(Clone, Debug, Default)]
pub(crate) struct FlowModel {
    /// The events, in evaluation order.
    pub events: Vec<FlowEvent>,
    /// The reads, in evaluation order.
    pub reads: Vec<FlowRead>,
    /// The definitions that can reach each read, in the order of the reads.
    pub uses: Vec<FlowUse>,
    /// The shadows, in evaluation order.
    pub shadows: Vec<FlowShadow>,
    /// The reads ordered by the name each reads, then by evaluation order: made by the first fill of the reads of one name.
    pub reads_by_name: Vec<KeyPlace<NameCode>>,
    /// The shadows ordered by the binding of each, then by evaluation order: made by the first fill of the shadows of one
    /// binding.
    pub shadows_by_binding: Vec<KeyPlace<Position>>,
    /// The reads ordered by the name expression of each: made by the first fill of the lookup path of one read.
    pub reads_by_node: Vec<KeyPlace<Position>>,
    /// Whether the interpretation reached its end.
    pub state: FlowState,
    /// The frames of the interpretation.
    pub frames: FrameModel,
    /// What the reach of a name is worked out with.
    pub reach: ReachModel,
    /// The last frame of each finished phase, by the phase's root.
    pub phase_frames: HashMap<Position, FrameCode, BuildHasherDefault<KeyHasher>>,
    /// The open alternatives, innermost last.
    pub branches: Vec<OpenBranch>,
    /// The finished arms of the open alternatives.
    pub arms: Vec<FrameCode>,
}

/// What the events in effect in schedule order are worked out with: buffers kept from one file to the next, cleared by the
/// fill that uses them.
#[derive(Clone, Debug, Default)]
pub(crate) struct ScheduleOrder {
    /// The latest event of each name in each namespace so far, by its place among the events.
    pub latest: HashMap<NamespaceKey, Count, BuildHasherDefault<KeyHasher>>,
    /// The reads that are evaluated after the whole schedule (bounds and defaults of type parameters): the place of each
    /// among the events, and its read.
    pub lazy: Vec<(Count, FlowRead)>,
    /// The rows so far, each with the schedule ordinal of its read.
    pub rows: Vec<(Integer, BindingResolutionRow)>,
}
