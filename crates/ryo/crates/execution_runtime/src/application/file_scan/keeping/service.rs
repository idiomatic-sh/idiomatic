//! The keeping of a scanned file's found rows: every word cell is copied to owned text while the file's words are lent, and a
//! finding is placed while the file's table is. A found row is looked up by its rule ordinal, and nothing else is done per rule.

use super::super::types::{EmitColumns, FileNotice, PlaceColumns, ScanReport};
use ryo_engine::domain::findings::types::{
    ColumnKinds, FoundCells, FoundRows, FoundTarget, WorkNotice,
};
use ryo_language::domain::facts::service::text;
use ryo_language::domain::facts::types::{CellWord, FileWords};
use ryo_language::domain::node_table::service::{element_count, implied_of, own_end, span_of};
use ryo_language::domain::node_table::types::{Implied, NodeTable};
use ryo_relations::domain::relations::service::{from_bits, location, node_file, node_position};
use ryo_relations::domain::relations::types::{Bits, CellKind, FileId, Locatedness, Span, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::scan_rows::types::{FrontierRow, KeptCell, KeptText, RowPlace, ScanRow};

/// Keeps the found rows of one file: every word cell is copied to owned text while the file's words are lent, and a finding is
/// placed while the file's table is.
pub(in crate::application::file_scan) struct RowKeeper<'a> {
    /// The columns of each selected rule's rows.
    pub(in crate::application::file_scan) columns: &'a EmitColumns,
    /// The columns that place each selected rule's rows.
    pub(in crate::application::file_scan) places: &'a PlaceColumns,
    /// The file being scanned.
    pub(in crate::application::file_scan) file: FileId,
    /// The file's table, which says where a node is.
    pub(in crate::application::file_scan) table: &'a NodeTable,
    /// Where the worker keeps what it found.
    pub(in crate::application::file_scan) report: &'a mut ScanReport,
    /// The first target a row did not fit the columns of.
    pub(in crate::application::file_scan) misfit: &'a mut Option<FoundTarget>,
}

impl RowKeeper<'_> {
    /// Where a finding is in the file: the stretch of the first of `places` that holds a stretch of the file, or a node of it.
    /// A node the parser implied is in no byte of the file and places nothing, as in the old engine's rendering.
    fn placed(&self, places: &[ColumnIndex], kinds: &ColumnKinds, row: &[CellWord]) -> RowPlace {
        places
            .iter()
            .find_map(|column| {
                let at = usize::from(*column);
                let (kind, cell) = (kinds.0.get(at)?, row.get(at)?);
                match from_bits(*kind, Bits(cell.0)) {
                    Value::Span(span) => (location(span) == Locatedness::Located).then_some(span),
                    Value::Node(node) => {
                        let position = node_position(node);
                        let within = usize::from(position) < element_count(self.table).0;
                        let written = implied_of(self.table, position) == Implied::No;
                        // SPIKE (ryo-fmx3.1, K): an element of an embedded segment (the expression of a string read as a
                        // type) places nothing, as in today's engine's rows: the oracle's row of such a finding has no
                        // line and no column.
                        let own = position < own_end(self.table);
                        (node_file(node) == self.file && within && written && own).then(|| {
                            let span = span_of(self.table, position);
                            Span {
                                start: span.start,
                                end: span.end,
                            }
                        })
                    }
                    Value::Symbol(_)
                    | Value::Name(_)
                    | Value::File(_)
                    | Value::Number(_)
                    | Value::Flag(_) => None,
                }
            })
            .map_or(RowPlace::Unplaced, RowPlace::At)
    }
}

impl FoundRows for RowKeeper<'_> {
    fn found(&mut self, target: FoundTarget, row: FoundCells<'_>, words: &FileWords) {
        let rule = match target {
            FoundTarget::Rule(rule) | FoundTarget::Frontier(rule, _) => rule,
        };
        let at = usize::try_from(rule.0).unwrap_or(usize::MAX);
        let kinds = match target {
            FoundTarget::Rule(_) => self.columns.0.get(at).map(|columns| &columns.emit),
            FoundTarget::Frontier(_, input) => self
                .columns
                .0
                .get(at)
                .and_then(|columns| columns.frontier.get(input.0)),
        };
        let Some(kinds) = kinds.filter(|kinds| kinds.0.len() == row.0.len()) else {
            if self.misfit.is_none() {
                *self.misfit = Some(target);
            }
            return;
        };
        let cells = row
            .0
            .iter()
            .zip(&kinds.0)
            .map(|(cell, kind)| match kind {
                CellKind::Symbol => KeptCell::Text(KeptText(text(words, *cell).0.into())),
                CellKind::Node
                | CellKind::File
                | CellKind::Span
                | CellKind::Number
                | CellKind::Flag => KeptCell::Number(*cell),
            })
            .collect();
        let file = self.file;
        match target {
            FoundTarget::Rule(rule) => {
                let places = self.places.0.get(at).map_or(&[][..], Vec::as_slice);
                let place = self.placed(places, kinds, row.0);
                self.report.found.push(ScanRow {
                    file,
                    rule,
                    place,
                    cells,
                });
            }
            FoundTarget::Frontier(rule, input) => self.report.frontier.push(FrontierRow {
                file,
                rule,
                input,
                cells,
            }),
        }
    }

    /// A rule's work notice in the file, kept with the file: the rule's rows are complete, and nothing here stops it.
    fn noticed(&mut self, notice: WorkNotice) {
        self.report.notices.push(FileNotice {
            file: self.file,
            notice,
        });
    }
}
