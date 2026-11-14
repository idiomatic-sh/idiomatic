//! What a `span_bounds` lowers to.

/// What a span-bounds lowers to: the span column.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpanBoundsParts {
    /// The span column.
    pub(crate) span: ryo_values::domain::plan_parts::types::ColumnIndex,
}
