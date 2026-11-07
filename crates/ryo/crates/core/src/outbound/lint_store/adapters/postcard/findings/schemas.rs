//! The findings document of the postcard store: its wire shapes and their conversion from and into the core's tables.

use std::borrow::Cow;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Offset;
use ryo_relations::domain::relations::types::Span;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::row_texts::types::Cell;
use ryo_values::domain::row_texts::types::PlacedColumn;
use ryo_values::domain::row_texts::types::PlacedLine;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_findings::types::FindingMade;
use ryo_values::domain::stored_findings::types::FindingPlace;
use ryo_values::domain::stored_findings::types::StoredCell;
use ryo_values::domain::stored_findings::types::StoredFinding;
use ryo_values::domain::stored_findings::types::StoredPlace;
use ryo_values::domain::stored_findings::types::StoredValue;
use super::schemas::{StringsSchema, TextSchema, file_number, named, text};
use super::super::tables::schemas::{TablesSchema};

/// A value a finding holds, typed: a file or a text by its place in the table of strings, a whole number, or a stretch of a file by
/// its two ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) enum CellSchema {
    /// A file, by its path.
    File(TextSchema),
    /// A text.
    Text(TextSchema),
    /// A whole number.
    Number(i64),
    /// A stretch of a file: where it starts and where it ends.
    Span(u32, u32),
}

/// A finding: one row for each, numbered by its place in the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct FindingSchema {
    /// The rule.
    pub(in crate::outbound::lint_store::adapters::postcard) rule: TextSchema,
    /// The file whose visit made it, by its place among the files; none for a rule that reads across files.
    pub(in crate::outbound::lint_store::adapters::postcard) file: Option<u32>,
    /// Its line and character; none when it has no place.
    pub(in crate::outbound::lint_store::adapters::postcard) place: Option<(u32, u32)>,
    /// What it is about; none when it reads as nothing.
    pub(in crate::outbound::lint_store::adapters::postcard) subject: Option<CellSchema>,
}

/// One value of a finding's key: one row for each, in the order of the findings and, in a finding, of its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct KeySchema {
    /// The finding, by its place among the findings.
    pub(in crate::outbound::lint_store::adapters::postcard) finding: u32,
    /// The value; none when it reads as nothing.
    pub(in crate::outbound::lint_store::adapters::postcard) value: Option<CellSchema>,
}

/// One column of a finding: one row for each, in the order of the findings and, in a finding, of its columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct ValueSchema {
    /// The finding, by its place among the findings.
    pub(in crate::outbound::lint_store::adapters::postcard) finding: u32,
    /// The column's heading.
    pub(in crate::outbound::lint_store::adapters::postcard) column: TextSchema,
    /// Its value; none when it reads as nothing.
    pub(in crate::outbound::lint_store::adapters::postcard) value: Option<CellSchema>,
}

/// The row of a typed value, its text gained by `strings` when it has some.
pub(in crate::outbound::lint_store::adapters::postcard) fn cell_schema<'a>(strings: &mut StringsSchema<'a>, cell: &'a Cell) -> CellSchema {
    match cell {
        Cell::File(path) => CellSchema::File(text(strings, path)),
        Cell::Text(each) => CellSchema::Text(text(strings, each)),
        Cell::Number(number) => CellSchema::Number(number.0),
        Cell::Span(span) => CellSchema::Span(span.start.0, span.end.0),
    }
}

/// The typed value a row names; a refusal when the table of strings has none of its text.
pub(in crate::outbound::lint_store::adapters::postcard) fn cell_from(strings: &[Cow<'_, str>], cell: CellSchema) -> Result<Cell, postcard::Error> {
    Ok(match cell {
        CellSchema::File(path) => Cell::File(named(strings, path)?),
        CellSchema::Text(each) => Cell::Text(named(strings, each)?),
        CellSchema::Number(number) => Cell::Number(Integer(number)),
        CellSchema::Span(start, end) => Cell::Span(Span {
            start: Offset(start),
            end: Offset(end),
        }),
    })
}

/// The rows of the findings, of their key values and of their headed columns, their texts named in `strings`.
pub(in crate::outbound::lint_store::adapters::postcard) fn findings_schema<'a>(
    strings: &mut StringsSchema<'a>,
    stored: &'a [StoredFinding],
) -> (Vec<FindingSchema>, Vec<KeySchema>, Vec<ValueSchema>) {
    let mut findings = Vec::with_capacity(stored.len());
    let mut finding_keys = Vec::new();
    let mut finding_values = Vec::new();
    for (place, each) in stored.iter().enumerate() {
        findings.push(FindingSchema {
            rule: text(strings, &each.rule.0),
            file: match each.made {
                FindingMade::InFile(file) => Some(file.0),
                FindingMade::AcrossFiles => None,
            },
            place: match each.place {
                FindingPlace::At(at) => Some((at.line.0, at.column.0)),
                FindingPlace::Unplaced => None,
            },
            subject: Option::<&Cell>::from(&each.subject).map(|cell| cell_schema(strings, cell)),
        });
        let finding = u32::try_from(place).unwrap_or(u32::MAX);
        for value in &each.key {
            finding_keys.push(KeySchema {
                finding,
                value: Option::<&Cell>::from(value).map(|cell| cell_schema(strings, cell)),
            });
        }
        for value in &each.values {
            finding_values.push(ValueSchema {
                finding,
                column: text(strings, &value.column.0),
                value: Option::<&Cell>::from(&value.value).map(|cell| cell_schema(strings, cell)),
            });
        }
    }
    (findings, finding_keys, finding_values)
}

/// The findings of the tables, with their columns, of a store of `files` files.
pub(in crate::outbound::lint_store::adapters::postcard) fn findings_from(
    tables: &TablesSchema<'_>,
    files: usize,
) -> Result<Vec<StoredFinding>, postcard::Error> {
    let strings = &tables.strings;
    let mut findings = Vec::with_capacity(tables.findings.len());
    for each in &tables.findings {
        findings.push(StoredFinding {
            rule: RuleId(named(strings, each.rule)?),
            made: match each.file {
                Some(file) => FindingMade::InFile(file_number(files, file)?),
                None => FindingMade::AcrossFiles,
            },
            place: each.place.map_or(FindingPlace::Unplaced, |(line, column)| {
                FindingPlace::At(StoredPlace {
                    line: PlacedLine(line),
                    column: PlacedColumn(column),
                })
            }),
            subject: StoredCell::from(
                each.subject
                    .map(|cell| cell_from(strings, cell))
                    .transpose()?,
            ),
            key: Vec::new(),
            values: Vec::new(),
        });
    }
    for each in &tables.finding_keys {
        let finding = findings
            .get_mut(usize::try_from(each.finding).unwrap_or(usize::MAX))
            .ok_or(postcard::Error::DeserializeBadEncoding)?;
        finding.key.push(StoredCell::from(
            each.value
                .map(|cell| cell_from(strings, cell))
                .transpose()?,
        ));
    }
    for each in &tables.finding_values {
        let finding = findings
            .get_mut(usize::try_from(each.finding).unwrap_or(usize::MAX))
            .ok_or(postcard::Error::DeserializeBadEncoding)?;
        finding.values.push(StoredValue {
            column: ColumnName(named(strings, each.column)?),
            value: StoredCell::from(
                each.value
                    .map(|cell| cell_from(strings, cell))
                    .transpose()?,
            ),
        });
    }
    Ok(findings)
}
