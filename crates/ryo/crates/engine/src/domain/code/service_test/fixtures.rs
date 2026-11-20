//! What the round trips of the code cases share: operands written through a constructor and read back through a reader.

use crate::domain::code::types::{CodeWord, Op};
use std::fmt::Debug;

/// StepOperands as a constructor wrote them and as a reader read them back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RoundTrip {
    /// The operation of the word written.
    pub(super) op: Op,
    /// The operands written.
    pub(super) wrote: String,
    /// The operands read back.
    pub(super) read: String,
}

/// Writes `operands` through `write` and reads the word back through `read`.
pub(super) fn round_trip<O: Copy + Debug>(
    operands: O,
    write: fn(O) -> CodeWord,
    read: fn(CodeWord) -> O,
) -> RoundTrip {
    let word = write(operands);
    RoundTrip {
        op: word.op,
        wrote: format!("{operands:?}"),
        read: format!("{:?}", read(word)),
    }
}

/// The operation of a round trip whose reader gave back what its constructor wrote; else both.
pub(super) fn agreed(trip: RoundTrip) -> Result<Op, String> {
    if trip.wrote == trip.read {
        Ok(trip.op)
    } else {
        Err(format!("wrote {}, read {}", trip.wrote, trip.read))
    }
}
