//! The services of the `lines` part: the line cursor of a source search, moved on to an occurrence's first byte.

use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::source_matches::types::{CarriageReturn, LineBytes, LineCursor, SourceBase};
use ryo_relations::domain::relations::types::Integer;

/// Moves the line cursor on to `to`, counting the lines passed into `line`; a carriage return pending at a stop is kept
/// so that the line feed after it is not counted twice.
pub(crate) fn advance_line(
    bytes: LineBytes<'_>,
    source: LineBytes<'_>,
    base: SourceBase,
    cursor: &mut LineCursor,
    to: LineCursor,
    line: &mut Integer,
    pending_cr: &mut CarriageReturn,
) -> Result<(), PortableOperatorFault> {
    for &byte in bytes
        .0
        .get(cursor.0..to.0)
        .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?
    {
        if *pending_cr == CarriageReturn::Pending {
            line.0 = line
                .0
                .checked_add(1)
                .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
            *pending_cr = CarriageReturn::Clear;
            if byte == b'\n' {
                continue;
            }
        }
        if byte == b'\r' {
            *pending_cr = CarriageReturn::Pending;
        } else if byte == b'\n' {
            line.0 = line
                .0
                .checked_add(1)
                .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        }
    }
    *cursor = to;
    if *pending_cr == CarriageReturn::Pending && source.0.get(base.0 + to.0) != Some(&b'\n') {
        line.0 = line
            .0
            .checked_add(1)
            .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        *pending_cr = CarriageReturn::Clear;
    }
    Ok(())
}
