//! The types of the `traversal` part of the `steps` concept: what one step or stage touches. Crate-private.

use crate::domain::steps::types::{DefinitionId, Register, StepProgram};

/// What one step or stage touches itself: the registers it reads, the registers it writes and the bodies it runs. The steps
/// of a body are not in it: a walk goes into `bodies`. Each list is in the order of the instruction's operands, and a register
/// named twice is there twice.
#[derive(Debug, Default)]
pub(crate) struct Touched<'a> {
    /// The registers it reads: its register operands, the keys of its memo, and the values a use of a definition binds.
    pub(crate) reads: Vec<Register>,
    /// The registers it writes: its outputs, and the registers it sets for a body to read (`element` and `from`).
    pub(crate) writes: Vec<Register>,
    /// The bodies it runs.
    pub(crate) bodies: Vec<TouchedBody<'a>>,
}

/// A body a step or a stage runs.
#[derive(Clone, Copy, Debug)]
pub(crate) enum TouchedBody<'a> {
    /// Steps written in place: an inline body, the edges of a derived walk, a slot of a fork.
    Inline(&'a StepProgram),
    /// A definition of the pack: a keyed use of it, or its table as the source of the rows a step or stage iterates.
    Definition(DefinitionId),
}
