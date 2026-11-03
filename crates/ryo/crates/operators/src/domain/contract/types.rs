//! Portable semantic declarations interpreted by the engine.

/// The source length used to validate a composed byte interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceByteLength(pub usize);

/// One portable semantic fault vocabulary for every borrowed operator call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableOperatorFault {
    /// The declared input column is absent.
    NoSuchColumn {
        /// Missing column index.
        index: u32,
    },
    /// A present cell is not a span.
    ShapeMismatch,
    /// A span-bound extraction received an invalid interval.
    InvalidSpanBounds,
    /// A composed span exceeds the source or reverses its interval.
    InvalidSpan,
}

/// The result of one pure operator meaning, before engine publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorAnswer {
    /// Caller-owned scratch was filled.
    Written,
    /// Checked half-open span endpoints.
    SpanBounds([ryo_relations::domain::relations::types::Value; 2]),
    /// A composed half-open span, including a declared absent span.
    SpanBetween(ryo_relations::domain::relations::types::Value),
}

/// A borrowed request for one pure operator meaning. Storage and publication stay with the engine.
pub enum OperatorCall<'a> {
    /// Copy selected columns in plan order, preserving duplicates.
    Project {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Ordered column indices.
        columns: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        /// Caller-owned scratch output.
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Append a resolved row suffix without acquiring or publishing it.
    Append {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Resolved suffix.
        appended: &'a [ryo_relations::domain::relations::types::Value],
        /// Caller-owned scratch output.
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Read checked endpoints of one span cell.
    SpanBounds {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Declared span column.
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
    /// Compose the start of one span with the end of another.
    SpanBetween {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Left span column.
        start: ryo_values::domain::plan_parts::types::ColumnIndex,
        /// Right span column.
        end: ryo_values::domain::plan_parts::types::ColumnIndex,
        /// Complete source byte length validated by the engine.
        source_len: SourceByteLength,
    },
}
