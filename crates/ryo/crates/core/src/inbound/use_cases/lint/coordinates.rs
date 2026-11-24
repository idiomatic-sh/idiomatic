//! Lazily index one source for SDK finding locations. Coordinates use the same
//! one-based Unicode character and line convention as the shipping report.
use crate::domain::finish::{
    service::{fill_line_starts, place_in},
    types::LineStarts,
};
use ryo_relations::domain::relations::types::Offset;
use ryo_values::domain::files::types::SourceView;

#[derive(Default)]
pub struct FindingCoordinates(LineStarts);

impl FindingCoordinates {
    /// Called while the file is borrowed. No bytes are retained by the report.
    #[must_use]
    pub fn at(&mut self, source: &[u8], offset: Offset) -> Option<(u32, u32)> {
        if offset.0 as usize > source.len() {
            return None;
        }
        let source = SourceView(source);
        if self.0.0.is_empty() {
            fill_line_starts(
                source,
                Offset(u32::try_from(source.0.len()).unwrap_or(u32::MAX)),
                &mut self.0,
            );
        }
        let point = place_in(source, &self.0, offset);
        Some((point.line.0, point.column.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finding_columns_count_characters_and_accept_an_eof_span() {
        let source = "éx\r\nβ".as_bytes();
        let mut locations = FindingCoordinates::default();
        assert_eq!(locations.at(source, Offset(2)), Some((1, 2)));
        assert_eq!(locations.at(source, Offset(5)), Some((2, 1)));
        assert_eq!(locations.at(source, Offset(7)), Some((2, 2)));
        assert_eq!(locations.at(source, Offset(8)), None);
    }
}
