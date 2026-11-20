//! Values of the schedule-order lookup: plain data, public fields, no constructor.
use crate::domain::binding_resolution::types::TypeParameter;
use crate::domain::binding_results::types::{EventKind, ReadContext};
use crate::domain::model::types::{BindingAnalysisState, BindingKey};
use ryo::languages::{Integer, Position, Symbol};
use std::collections::{HashMap, HashSet};

/// One event a lookup of a name finds in one namespace on the reference's lookup path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LookupRow {
    /// The read or deletion occurrence.
    pub reference: Position,
    /// The name read or deleted.
    pub name: Symbol,
    /// The occurrence of the event; a type parameter's defines id.
    pub event: Position,
    /// What the event does.
    pub kind: EventKind,
    /// The namespace on the path that holds the event.
    pub scope: Position,
    /// The namespace whose code holds the event before `global`/`nonlocal` routing.
    pub lexical: Position,
    /// The namespace's place on the path: 0 for the first.
    pub depth: Integer,
    /// The event's schedule ordinal; 0 for a type parameter.
    pub ordinal: Integer,
    /// The reference's value, type or deletion context.
    pub context: ReadContext,
}

/// The absent link of an event chain.
pub(super) const NONE: u32 = u32::MAX;

/// Capacity-retaining lookup of the current file, made on the first read of the relation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BindingLookupScratch {
    /// Rows by the reference's schedule ordinal, then depth, then most recent event first.
    pub rows: Vec<LookupRow>,
    /// Whether the current file has been looked up.
    pub(super) state: BindingAnalysisState,
    /// The latest event of each (namespace, name), as an index into the analysis events.
    pub(super) latest: HashMap<BindingKey, u32>,
    /// For each event index, the previous event of the same (namespace, name), or `NONE`.
    pub(super) previous: Vec<u32>,
    /// The (namespace, name) pairs that hold an event routed there by `global` or `nonlocal`.
    pub(super) routed: HashSet<BindingKey>,
    /// The first `global` or `nonlocal` declaration of each (declaring namespace, name).
    pub(super) declared: HashMap<BindingKey, u32>,
    /// The file's type parameters.
    pub(super) parameters: Vec<TypeParameter>,
    /// Lazy reads (bounds and defaults of type parameters), looked up at the end of the schedule: event index and context.
    pub(super) lazy: Vec<(usize, ReadContext)>,
    /// The schedule ordinal of each row's reference, for ordering lazy reads among the others.
    pub(super) order: Vec<Integer>,
    /// Scratch for that ordering, kept between files.
    pub(super) sorting: Vec<(Integer, LookupRow)>,
    /// Scratch of one namespace walk: the code blocks whose binding in effect has been found.
    pub(super) found: Vec<Position>,
    /// Scratch of one namespace walk: per code block, the ended exception handler whose events are being passed over.
    pub(super) ended: Vec<(Position, Position)>,
}
