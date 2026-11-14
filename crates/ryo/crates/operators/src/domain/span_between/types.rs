//! What a `span_between` lowers to.

/// What a span-between lowers to: the start and end span columns.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpanBetweenParts {
    /// The span supplying the start.
    pub(crate) start: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The span supplying the end.
    pub(crate) end: ryo_values::domain::plan_parts::types::ColumnIndex,
}
