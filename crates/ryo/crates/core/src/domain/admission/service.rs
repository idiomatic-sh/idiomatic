//! The `admission` service: checks of the numbers a language wrote against the file they were written for, with no IO and no ports.

use ryo_language::domain::elements::service::{
    diagnostic_count, end_at, end_of, positions, start_at,
};
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::constants::{DIAGNOSTIC_BASE, IMPLIED, NO_SPAN, TOKEN};
use ryo_relations::domain::relations::service::{node_file, node_position};
use ryo_relations::domain::relations::types::{Bits, CellKind, ColumnsView, FileId, NodeId};
use ryo_values::domain::files::types::FileBytes;

use super::errors::AdmissionError;
use super::types::{ByteLength, ElementTotal, Limits, SpanContainment, StoredNumbers};

/// Whether the span stored as `bits` (the start in the high half, the end in the low) lies within the file.
fn span_within(bits: Bits, limits: &Limits) -> SpanContainment {
    let (start, end) = (bits.0 >> 32_i32, bits.0 & 0xffff_ffff);
    if (start == u64::from(NO_SPAN.start.0) && end == u64::from(NO_SPAN.end.0))
        || (start <= end && end <= limits.length.0)
    {
        SpanContainment::Within
    } else {
        SpanContainment::Outside
    }
}

/// Admits a column of `kind`: every cell of a span lies within the file, every cell of a node is a place of the file's table in this
/// file, every cell of a file is the file visited. Other kinds are not proven by the file and are admitted.
///
/// # Errors
///
/// The first cell that is not within the file.
pub(crate) fn admit_column(
    kind: CellKind,
    words: &StoredNumbers<'_>,
    limits: &Limits,
) -> Result<(), AdmissionError> {
    match kind {
        CellKind::Span => {
            if words
                .0
                .iter()
                .all(|bits| span_within(Bits(*bits), limits) == SpanContainment::Within)
            {
                Ok(())
            } else {
                Err(AdmissionError::SpanOutsideFile)
            }
        }
        CellKind::Node => {
            for bits in words.0 {
                let node = NodeId(*bits);
                if node_file(node) != limits.file {
                    return Err(AdmissionError::NodeOfAnotherFile);
                }
                let place = node_position(node).0;
                let outside = match place.checked_sub(DIAGNOSTIC_BASE) {
                    Some(report) => u64::from(report) >= limits.diagnostics.0,
                    None => u64::from(place) >= limits.elements.0,
                };
                if outside {
                    return Err(AdmissionError::NodeOutsideTable);
                }
            }
            Ok(())
        }
        CellKind::File => {
            if words
                .0
                .iter()
                .all(|bits| FileId(u32::try_from(*bits).unwrap_or(u32::MAX)) == limits.file)
            {
                Ok(())
            } else {
                Err(AdmissionError::FileNotVisited)
            }
        }
        CellKind::Symbol | CellKind::Number | CellKind::Flag => Ok(()),
    }
}

/// Admits the spans of the elements of the file's own tree: each ends no earlier than it starts and no later than the file.
///
/// # Errors
///
/// `SpanOutsideFile` for the first element whose span is not within the file.
pub(crate) fn admit_elements(elements: &Elements, limits: &Limits) -> Result<(), AdmissionError> {
    for position in positions(elements) {
        let (start, end) = (start_at(elements, position).0, end_at(elements, position).0);
        let flags = elements
            .flags
            .get(usize::try_from(position.0).unwrap_or(usize::MAX))
            .map_or(0, |flags| flags.0);
        let implied = flags & IMPLIED.0 != 0;
        if implied && flags & TOKEN.0 == 0 && start == NO_SPAN.start.0 && end == NO_SPAN.end.0 {
            continue;
        }
        if implied || start > end || u64::from(end) > limits.length.0 {
            return Err(AdmissionError::SpanOutsideFile);
        }
    }
    Ok(())
}

/// What the output of a language for the file `file`, with these `elements` and these `bytes`, must lie within.
#[must_use]
pub(crate) fn limits_of(elements: &Elements, bytes: &FileBytes, file: FileId) -> Limits {
    Limits {
        file,
        elements: ElementTotal(u64::from(end_of(elements).0)),
        diagnostics: ElementTotal(u64::try_from(diagnostic_count(elements).0).unwrap_or(u64::MAX)),
        length: ByteLength(u64::try_from(bytes.0.len()).unwrap_or(u64::MAX)),
    }
}

/// Admits every column of the rows of a relation the file wrote.
///
/// # Errors
///
/// The first cell that is not within the file.
pub(crate) fn admit_columns(view: &ColumnsView<'_>, limits: &Limits) -> Result<(), AdmissionError> {
    for (kind, column) in view.kinds.iter().zip(view.columns) {
        admit_column(*kind, &StoredNumbers(&column.0), limits)?;
    }
    Ok(())
}
