//! Values of schedule-order name resolution: plain data, public fields, no constructor.
use crate::domain::binding_results::types::{EventKind, ReadContext, TypeParameter};
use crate::domain::model::types::{BindingAnalysisState, BindingKey};
use ryo::languages::{Integer, Position};
use std::collections::HashMap;

/// The place of a namespace on a reference's lookup path: 0 for the first; namespaces sharing a step share a depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Depth(pub Integer);

/// One namespace's answer for one read or deletion: the event in effect there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Resolution {
    /// The read or deletion occurrence.
    pub reference: Position,
    /// The occurrence of the event in effect; a type parameter's defines id.
    pub event: Position,
    /// What the event does.
    pub kind: EventKind,
    /// The namespace on the path that holds the event.
    pub scope: Position,
    /// The namespace whose code holds the event before `global`/`nonlocal` routing.
    pub lexical: Position,
    /// The namespace's place on the path.
    pub depth: Depth,
    /// The event's schedule ordinal; 0 for a type parameter.
    pub ordinal: Integer,
    /// The reference's value, type or deletion context.
    pub context: ReadContext,
}

/// Capacity-retaining resolution of the current file, made on the first read of the relation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BindingResolutionScratch {
    /// Rows by the reference's schedule ordinal, then depth.
    pub resolutions: Vec<Resolution>,
    /// Whether the current file has been resolved.
    pub(super) state: BindingAnalysisState,
    /// The latest event of each (namespace, name), as an index into the analysis events.
    pub(super) latest: HashMap<BindingKey, usize>,
    /// The file's type parameters.
    pub(super) parameters: Vec<TypeParameter>,
    /// Lazy reads (bounds and defaults of type parameters), resolved at the end of the schedule: event index and context.
    pub(super) lazy: Vec<(usize, ReadContext)>,
    /// The schedule ordinal of each row's reference, for ordering lazy reads among the others.
    pub(super) order: Vec<Integer>,
    /// Scratch for that ordering, kept between files.
    pub(super) sorting: Vec<(Integer, Resolution)>,
}
