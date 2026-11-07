//! Public vocabulary of raw-format-field.
use super::item::RawFormatFieldColumn;
use crate::domain::relations::constants::{CONVERSION, ORDINAL};
/// Relation catalog name.
#[must_use]
pub fn name() -> &'static str {
    "raw-format-field"
}
/// Relation promise, independent of any diagnostic rule.
#[must_use]
pub fn summary() -> &'static str {
    "Ordered fields from complete raw-token parses, retaining canonical unique flags without conversion policy; malformed segments contribute no fields."
}
/// Public name of one typed column.
#[must_use]
pub fn column_name(column: RawFormatFieldColumn) -> &'static str {
    match column {
        RawFormatFieldColumn::Id => "id",
        RawFormatFieldColumn::Segment => "segment",
        RawFormatFieldColumn::SegmentOrdinal => "segment-ordinal",
        RawFormatFieldColumn::Ordinal => ORDINAL,
        RawFormatFieldColumn::SegmentFieldOrdinal => "segment-field-ordinal",
        RawFormatFieldColumn::ArgumentKind => "argument-kind",
        RawFormatFieldColumn::MappingKey => "mapping-key",
        RawFormatFieldColumn::Conversion => CONVERSION,
        RawFormatFieldColumn::WidthMode => "width-mode",
        RawFormatFieldColumn::PrecisionMode => "precision-mode",
        RawFormatFieldColumn::File => "file",
        RawFormatFieldColumn::Span => "span",
        RawFormatFieldColumn::Flags => "flags",
    }
}
