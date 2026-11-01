//! Public vocabulary of raw-format-field.
use super::item::RawFormatFieldColumn;
/// Relation catalog name.
pub fn name() -> &'static str {
    "raw-format-field"
}
/// Relation promise, independent of any diagnostic rule.
pub fn summary() -> &'static str {
    "Ordered fields from complete raw-token parses; malformed segments contribute no fields."
}
/// Public name of one typed column.
pub fn column_name(column: RawFormatFieldColumn) -> &'static str {
    match column {
        RawFormatFieldColumn::Id => "id",
        RawFormatFieldColumn::Segment => "segment",
        RawFormatFieldColumn::SegmentOrdinal => "segment-ordinal",
        RawFormatFieldColumn::Ordinal => "ordinal",
        RawFormatFieldColumn::SegmentFieldOrdinal => "segment-field-ordinal",
        RawFormatFieldColumn::ArgumentKind => "argument-kind",
        RawFormatFieldColumn::MappingKey => "mapping-key",
        RawFormatFieldColumn::Conversion => "conversion",
        RawFormatFieldColumn::WidthMode => "width-mode",
        RawFormatFieldColumn::PrecisionMode => "precision-mode",
        RawFormatFieldColumn::File => "file",
        RawFormatFieldColumn::Span => "span",
    }
}
