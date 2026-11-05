//! The `keeping` service: free functions named by verb that turn what a run came to into what the next
//! can use, with no IO and no ports.

use ryo_language::domain::words::service::{
    decode_kind, decode_places, decode_value, encode_kind, encode_places, encode_value,
};
use ryo_relations::domain::relations::service::{node_file, node_id, node_position};
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{Bits, NamePlace, Offset, Origin, Value, Words};

use super::types::{Configuration, DigestInput, DigestPart, Remap};

use crate::domain::finish::service::{column_cell, line_cell};
use crate::domain::render::service::{finding_parts, key_count, row_cells};
use crate::domain::render::types::{KeyCount, Rendered, RowNumber, RowOrder};
use crate::domain::table_words::service::unpacked;
use ryo_values::domain::program::types::ColumnName;
use ryo_values::domain::records::service::{keep, lent, owned};
use ryo_values::domain::records::types::RenderedText;
use ryo_values::domain::records::types::{
    Cell, CellText, PlacedColumn, PlacedLine, PlacedRow, PlacedRule, RowEnds, RowsText,
    RowsTextView, TextRange,
};
use ryo_values::domain::records::types::{FindingRows, RowPlacing, RowTotal};
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stores::types::{
    FindingMade, FindingPlace, StoredCell, StoredFinding, StoredPlace, StoredValue,
};
use {
    crate::domain::finish::types::ColumnNumber, crate::domain::finish::types::FileLine,
    crate::domain::finish::types::Point, ryo_values::domain::runs::types::RuleNumber,
};

/// The findings of a file as a record keeps them.
#[must_use]
pub(crate) fn rows_of(rendered: &Rendered) -> FindingRows {
    FindingRows {
        text: RowsText(rendered.text.as_str().into()),
        cells: rendered.cells.clone(),
        ends: rendered.ends.clone(),
        order: rendered
            .order
            .iter()
            .map(|each| RowPlacing {
                rule: PlacedRule(each.rule.0),
                line: PlacedLine(each.line.0),
                column: PlacedColumn(each.column.0),
                row: PlacedRow(each.row.0),
            })
            .collect(),
        total: rendered.total,
    }
}

/// The findings of a file, as a record kept them, ready to leave.
#[must_use]
pub(crate) fn rendered_of(rows: &FindingRows) -> Rendered {
    Rendered {
        text: RenderedText(rows.text.0.to_string()),
        cells: rows.cells.clone(),
        ends: rows.ends.clone(),
        order: rows
            .order
            .iter()
            .map(|each| RowOrder {
                rule: RuleNumber(each.rule.0),
                line: FileLine(each.line.0),
                column: ColumnNumber(each.column.0),
                row: RowNumber(each.row.0),
            })
            .collect(),
        total: rows.total,
        keys: Vec::new(),
    }
}

/// Where a value of the file goes in the run that uses its words, or nothing when it points at another file
/// or at text that is not the file's: such words cannot be moved, and the file is read again instead.
fn moved(value: Value, map: &Remap) -> Option<Value> {
    Some(match value {
        Value::File(file) if file == map.from_file => Value::File(map.to_file),
        Value::Node(node) if node_file(node) == map.from_file => {
            Value::Node(node_id(map.to_file, node_position(node)))
        }
        Value::File(_) | Value::Node(_) | Value::Symbol(_) => return None,
        // A name is its hash and the number of its place among the places of its table, which `moved_place` moves.
        Value::Name(_) | Value::Span(_) | Value::Number(Integer(_)) | Value::Flag(_) => value,
    })
}

/// Where the text of a name of the file goes in the run that uses its words, or nothing when it is not the file's.
fn moved_place(place: NamePlace, map: &Remap) -> Option<NamePlace> {
    if place.file != map.from_file {
        return None;
    }
    let origin = match place.origin {
        Origin::Source => Origin::Source,
        Origin::Derived(worker) if worker == map.from_worker => Origin::Derived(map.to_worker),
        Origin::Derived(_) => return None,
    };
    let (start, end) = match place.origin {
        Origin::Source => (place.start.0, place.end.0),
        Origin::Derived(_) => (
            place
                .start
                .0
                .checked_sub(map.from_base.0)?
                .checked_add(map.to_base.0)?,
            place
                .end
                .0
                .checked_sub(map.from_base.0)?
                .checked_add(map.to_base.0)?,
        ),
    };
    Some(NamePlace {
        origin,
        file: map.to_file,
        start: Offset(start),
        end: Offset(end),
    })
}

/// The words packed in `packed` as they are in the run that uses them, moved as `map` says, written onto `out` as they are read, so
/// no table is held apart. Nothing when they cannot be moved; a table cut short is left out, with those after it.
#[must_use]
pub(crate) fn remap_packed(packed: &PackedWords, map: &Remap, out: &mut Words) -> Option<()> {
    let mut words = unpacked(packed);
    let mut places = Vec::new();
    while let Some(Bits(columns)) = words.next() {
        let start = out.0.len();
        let Some(Bits(rows)) = words.next() else {
            return Some(());
        };
        out.0.extend([columns, rows]);
        for _ in 0..columns {
            let Some(kind) = decode_kind(&mut words) else {
                out.0.truncate(start);
                return Some(());
            };
            encode_kind(kind, out);
        }
        places.clear();
        if decode_places(&mut words, &mut places).is_none() {
            out.0.truncate(start);
            return Some(());
        }
        for place in &mut places {
            *place = moved_place(*place, map)?;
        }
        encode_places(&places, out);
        for _ in 0..rows {
            for _ in 0..columns {
                let Some(value) = decode_value(&mut words) else {
                    out.0.truncate(start);
                    return Some(());
                };
                encode_value(moved(value, map)?, out);
            }
        }
    }
    Some(())
}

/// Writes a string with its length, so strings run together stay apart.
fn put(out: &mut DigestInput, text: DigestPart<'_>) {
    let text = text.0;
    let out = &mut out.0;
    out.extend_from_slice(&u64::try_from(text.len()).unwrap_or(u64::MAX).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
}

/// What the findings of rules depend on of the rules themselves and of the relations the host declares, as bytes to digest.
#[must_use]
pub(crate) fn configuration_bytes(configuration: &Configuration<'_>) -> DigestInput {
    let mut out = DigestInput::default();
    out.0.extend_from_slice(
        &u64::try_from(configuration.rules.len())
            .unwrap_or(0)
            .to_le_bytes(),
    );
    for rule in configuration.rules {
        put(&mut out, DigestPart(&rule.id.0));
        put(
            &mut out,
            DigestPart(match rule.applicability {
                ryo_values::domain::runs::types::RuleApplicability::Always => "always",
                ryo_values::domain::runs::types::RuleApplicability::AcceptedSyntax => {
                    "accepted-syntax"
                }
            }),
        );
        put(&mut out, DigestPart(&rule.query.root.id.0));
        put(&mut out, DigestPart(&rule.query.root.text));
        out.0.extend_from_slice(
            &u64::try_from(rule.query.modules.len())
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
        );
        for (id, source) in &rule.query.modules {
            put(&mut out, DigestPart(&id.0));
            put(&mut out, DigestPart(&source.text));
        }
        put(&mut out, DigestPart(&rule.spelled.0));
        // Ordinary rules retain their historical bytes; declared dependency contracts
        // carry a versioned frame including producer IDs, output names and schema.
        out.0.extend_from_slice(
            &ryo_values::domain::observations::service::identity(&rule.observations).0,
        );
        for globs in [&rule.include, &rule.exclude] {
            out.0
                .extend_from_slice(&u64::try_from(globs.len()).unwrap_or(0).to_le_bytes());
            for glob in globs {
                put(&mut out, DigestPart(&glob.0));
            }
        }
    }
    out.0.extend_from_slice(
        &u64::try_from(configuration.hosted.len())
            .unwrap_or(0)
            .to_le_bytes(),
    );
    for line in configuration.hosted {
        put(&mut out, DigestPart(&line.0));
    }
    out
}

/// The findings `rows` holds, made as `made` says, each cell owned. A run keeps only the rows of findings; a row that is not laid out
/// as a finding's is left out.
#[must_use]
pub(crate) fn findings_of(rows: &FindingRows, made: FindingMade) -> Vec<StoredFinding> {
    let text = RowsTextView(&rows.text.0);
    let cell_of =
        |cell: &Option<Cell<TextRange>>| StoredCell::from(cell.map(|kept| owned(lent(text, kept))));
    rows.order
        .iter()
        .filter_map(|placing| {
            let found =
                finding_parts(row_cells(&rows.cells, &rows.ends, RowNumber(placing.row.0)))?;
            let values = found
                .columns
                .chunks_exact(2)
                .map(|pair| match pair {
                    [Some(Cell::Text(heading)), value] => Some(StoredValue {
                        column: ColumnName(kept_text(text, *heading).0.into()),
                        value: cell_of(value),
                    }),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?;
            Some(StoredFinding {
                rule: RuleId(kept_text(text, *found.rule).0.into()),
                made,
                place: if placing.line.0 == 0 {
                    FindingPlace::Unplaced
                } else {
                    FindingPlace::At(StoredPlace {
                        line: placing.line,
                        column: placing.column,
                    })
                },
                subject: cell_of(found.subject),
                key: found.key.iter().map(cell_of).collect(),
                values,
            })
        })
        .collect()
}

/// The findings as the rows they are written as, each rule numbered by its place among `ids`; none when a finding's rule is not one
/// of them.
#[must_use]
pub(crate) fn rows_from(findings: &[&StoredFinding], ids: &[RuleId]) -> Option<FindingRows> {
    let mut text = RenderedText::default();
    let mut cells = Vec::new();
    let mut ends = Vec::with_capacity(findings.len());
    let mut order = Vec::with_capacity(findings.len());
    for (row, finding) in findings.iter().enumerate() {
        let rule = ids.iter().position(|id| *id.0 == *finding.rule.0)?;
        let id = keep(&mut text, Cell::Text(CellText(&finding.rule.0)));
        cells.push(Some(id));
        let subject =
            Option::<&Cell>::from(&finding.subject).map(|cell| keep(&mut text, lent_owned(cell)));
        cells.push(subject);
        cells.push(Some(key_count(KeyCount(finding.key.len()))));
        for each in &finding.key {
            let kept = Option::<&Cell>::from(each).map(|cell| keep(&mut text, lent_owned(cell)));
            cells.push(kept);
        }
        for each in &finding.values {
            let heading = keep(&mut text, Cell::Text(CellText(&each.column.0)));
            cells.push(Some(heading));
            let kept =
                Option::<&Cell>::from(&each.value).map(|cell| keep(&mut text, lent_owned(cell)));
            cells.push(kept);
        }
        let at = match finding.place {
            FindingPlace::At(at) => at,
            FindingPlace::Unplaced => StoredPlace {
                line: PlacedLine(0),
                column: PlacedColumn(0),
            },
        };
        let (line, column) = (at.line, at.column);
        let place = Point {
            line: FileLine(line.0),
            column: ColumnNumber(column.0),
        };
        cells.push(line_cell(place));
        cells.push(column_cell(place));
        ends.push(u32::try_from(cells.len()).unwrap_or(u32::MAX));
        order.push(RowPlacing {
            rule: PlacedRule(u32::try_from(rule).unwrap_or(u32::MAX)),
            line,
            column,
            row: PlacedRow(u32::try_from(row).unwrap_or(u32::MAX)),
        });
    }
    Some(FindingRows {
        text: RowsText(text.0.into()),
        cells,
        ends: RowEnds(ends),
        order,
        total: RowTotal(u64::try_from(findings.len()).unwrap_or(u64::MAX)),
    })
}

/// An owned cell, lent from itself.
fn lent_owned(cell: &Cell) -> Cell<CellText<'_>> {
    match cell {
        Cell::File(path) => Cell::File(CellText(path)),
        Cell::Text(text) => Cell::Text(CellText(text)),
        Cell::Number(number) => Cell::Number(*number),
        Cell::Span(span) => Cell::Span(*span),
    }
}

/// The text a kept text cell holds in `text`, the text of its rows.
fn kept_text(text: RowsTextView<'_>, range: TextRange) -> CellText<'_> {
    match lent(text, Cell::Text(range)) {
        Cell::File(piece) | Cell::Text(piece) => piece,
        Cell::Number(_) | Cell::Span(_) => CellText::default(),
    }
}
