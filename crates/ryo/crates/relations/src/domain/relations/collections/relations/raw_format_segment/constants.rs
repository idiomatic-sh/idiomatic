//! Public vocabulary of raw-format-segment.
use super::item::RawFormatSegmentColumn;
/// Relation catalog name.
#[must_use]
pub fn name() -> &'static str {
    "raw-format-segment"
}
/// Relation promise, independent of any diagnostic rule.
#[must_use]
pub fn summary() -> &'static str {
    "One outcome per raw string token, including empty and malformed bodies."
}
/// Public name of one typed column.
#[must_use]
pub fn column_name(column: RawFormatSegmentColumn) -> &'static str {
    match column {
        RawFormatSegmentColumn::Id => "id",
        RawFormatSegmentColumn::Segment => "segment",
        RawFormatSegmentColumn::SegmentOrdinal => "segment-ordinal",
        RawFormatSegmentColumn::Dialect => "dialect",
        RawFormatSegmentColumn::Outcome => "outcome",
        RawFormatSegmentColumn::ErrorCategory => "error-category",
        RawFormatSegmentColumn::ErrorCharacter => "error-character",
        RawFormatSegmentColumn::ErrorIndex => "error-index",
        RawFormatSegmentColumn::PositionalConsumption => "positional-consumption",
        RawFormatSegmentColumn::MappingPresent => "mapping-present",
        RawFormatSegmentColumn::DynamicWidth => "dynamic-width",
        RawFormatSegmentColumn::DynamicPrecision => "dynamic-precision",
        RawFormatSegmentColumn::File => "file",
        RawFormatSegmentColumn::Span => "span",
    }
}
