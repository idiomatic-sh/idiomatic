//! The types of the `closures` part: what a closure's walk asks for its next elements.

use crate::domain::lowering::flattening::types::FlatAtom;
use crate::domain::lowering::planning::types::Emitter;
use crate::domain::steps::types::{MadeColumn, StepOperand};

/// One bound column of a use, with its cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoundCell {
    /// The column.
    pub(crate) column: MadeColumn,
    /// Its cell.
    pub(crate) value: StepOperand,
}

/// The writer of one atom's accesses: the emitter's, handed to a walk that writes its edges in place.
pub(crate) type AtomWriter = fn(&mut Emitter<'_>, &FlatAtom);

/// The two columns of the edges a walk's step reads: the one the element in hand is asked by, and the one a row gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EdgeEnds {
    /// The column asked by the element in hand.
    pub(crate) asked: MadeColumn,
    /// The column a row gives: the next element.
    pub(crate) gives: MadeColumn,
}
