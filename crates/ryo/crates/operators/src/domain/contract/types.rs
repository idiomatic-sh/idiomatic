//! Borrowed semantic contracts. The engine owns every acquired resource and output.

/// The source length used to validate a composed byte interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceByteLength(pub usize);

/// One portable semantic fault vocabulary for every borrowed operator call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableOperatorFault {
    /// The engine stopped candidate acquisition or downstream delivery.
    Interrupted,
    /// The declared input column is absent.
    NoSuchColumn {
        /// Missing column index.
        index: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
    /// A present cell is not a span.
    ShapeMismatch,
    /// A span-bound extraction received an invalid interval.
    InvalidSpanBounds,
    /// A composed span exceeds the source or reverses its interval.
    InvalidSpan,
    /// An occurrence cannot be represented inside the validated physical source.
    InvalidSourceMatchSpan,
    /// A required named regular-expression capture is absent.
    MissingCapture,
    /// The supplied semantic options are not a valid operation.
    WrongOperation,
    /// A requested table key is absent from a table.
    UnboundKey,
    /// A language provider cannot render the requested syntax.
    UnsupportedRender,
    /// A resource the operation needs is not bound to the invocation, or a bound value does not fit it.
    WrongBinding,
    /// A parameter the operation reads has no binding.
    Unbound,
    /// A row the operation reads was not produced.
    NotRun,
}
