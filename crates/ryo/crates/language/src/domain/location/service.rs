//! Source coordinates from an index built once per requested file, with no IO.

use crate::domain::elements::service::{engine, engine_mut};
use crate::domain::elements::types::Elements;
use crate::domain::relations::types::{Integer, Offset};
use crate::domain::symbols::types::ByteSlice;

/// Builds the index only when requested. CRLF is one break; lone CR and LF also end lines.
pub(crate) fn index(elements: &mut Elements, source: ByteSlice<'_>) {
    let starts = &mut engine_mut(elements).line_starts;
    if !starts.is_empty() {
        return;
    }
    starts.push(Offset(0));
    let mut at = 0;
    while let Some(byte) = source.0.get(at) {
        let newline = matches!(*byte, b'\r' | b'\n');
        if *byte == b'\r' && source.0.get(at + 1) == Some(&b'\n') {
            at += 1;
        }
        at += 1;
        if newline {
            starts.push(Offset(u32::try_from(at).unwrap_or(u32::MAX)));
        }
    }
}

/// The one-based line of an endpoint, found in logarithmic time without rescanning the source.
#[must_use]
pub(crate) fn line(elements: &Elements, offset: Offset) -> Integer {
    Integer(
        i64::try_from(
            engine(elements)
                .line_starts
                .partition_point(|start| *start <= offset),
        )
        .unwrap_or(i64::MAX),
    )
}
