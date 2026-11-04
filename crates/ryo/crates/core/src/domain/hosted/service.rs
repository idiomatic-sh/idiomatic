//! The `hosted` service: free functions over the relations the host declares, with no IO and no ports.

use ryo_language::domain::symbols::service::{intern_derived, query_intern_derived};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::{QuerySymbols, Symbols};
use ryo_relations::domain::relations::types::{CellKind, FileId, Value};
use ryo_relations::domain::terms::types::NameText;

#[cfg(test)]
use super::constants::tab;
use super::types::NumberedFiles;
#[cfg(test)]
use crate::domain::keeping::types::HostedText;
use crate::domain::table_model::service::{push_row, reset};

use ryo_values::domain::hosted::service::kind_of;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::records::types::Cell;

/// Historical readable presentation, retained to prove the typed identity is independent of its delimiters.
#[cfg(test)]
#[must_use]
pub(crate) fn lines(relations: &[HostRelation]) -> Vec<HostedText> {
    let mut lines = Vec::new();
    for relation in relations {
        let columns: Vec<String> = relation
            .columns
            .iter()
            .map(|column| format!("{}:{:?}", column.name.0, column.kind))
            .collect();
        let head = [relation.name.0.to_string(), columns.join(tab())];
        lines.push(HostedText(head.join(tab()).into()));
        for row in &relation.rows {
            let cells: Vec<String> = row
                .0
                .iter()
                .map(|cell| match cell {
                    Cell::File(path) => format!("file:{path}"),
                    Cell::Text(text) => format!("text:{text}"),
                    Cell::Number(number) => format!("number:{}", number.0),
                    Cell::Span(span) => format!("span:{}:{}", span.start.0, span.end.0),
                })
                .collect();
            lines.push(HostedText(cells.join(tab()).into()));
        }
    }
    lines
}

/// The rows of a relation as a table, its text interned in `symbols`. A file is its number among `files`; a row naming a file the run
/// does not list is no row, as a file that is not there is no file.
#[must_use]
pub(crate) fn table_of(
    relation: &HostRelation,
    files: NumberedFiles<'_>,
    symbols: &mut Symbols,
) -> Table {
    table_with(relation, files, |text| {
        intern_derived(symbols, ByteSlice(&[]), NameText(text))
    })
}

/// Materialize a declared request source in the query's derived-name pool.
/// Source-name interning remains unavailable to semantic operators.
pub(crate) fn table_of_query(
    relation: &HostRelation,
    files: NumberedFiles<'_>,
    symbols: &mut QuerySymbols<'_>,
) -> Table {
    table_with(relation, files, |text| {
        query_intern_derived(symbols, ByteSlice(&[]), NameText(text))
    })
}

fn table_with(
    relation: &HostRelation,
    files: NumberedFiles<'_>,
    mut symbol: impl FnMut(&str) -> ryo_relations::domain::terms::types::Symbol,
) -> Table {
    let mut table = Table::default();
    let kinds: Vec<CellKind> = relation
        .columns
        .iter()
        .map(|column| kind_of(column.kind))
        .collect();
    reset(&mut table, &kinds);
    'rows: for row in &relation.rows {
        let mut cells = Vec::with_capacity(row.0.len());
        for cell in &row.0 {
            cells.push(match cell {
                Cell::File(path) => {
                    let Ok(place) = files.0.binary_search_by(|file| (*file.0).cmp(&**path)) else {
                        continue 'rows;
                    };
                    Value::File(FileId(u32::try_from(place).unwrap_or(u32::MAX)))
                }
                Cell::Text(text) => Value::Symbol(symbol(text)),
                Cell::Number(number) => Value::Number(*number),
                // No column the host declares holds a span: the SDK refuses one, and a row that still has one is no row.
                Cell::Span(_) => continue 'rows,
            });
        }
        push_row(&mut table, &cells);
    }
    table
}

use ryo_values::domain::rows::types::Table;
/// Versioned typed identity of hosted facts. Presentation lines are deliberately not an encoding:
/// arbitrary text may itself contain tabs, newlines or the prefixes printed by `lines`.
#[must_use]
pub(crate) fn identity(relations: &[HostRelation]) -> crate::domain::keeping::types::DigestInput {
    use ryo_values::domain::hosted::types::HostKind;
    fn count(out: &mut Vec<u8>, value: usize) {
        out.extend_from_slice(&u64::try_from(value).unwrap_or(u64::MAX).to_le_bytes());
    }
    fn text(out: &mut Vec<u8>, value: &str) {
        count(out, value.len());
        out.extend_from_slice(value.as_bytes());
    }
    let mut out = b"ryo-hosted-facts-v1".to_vec();
    count(&mut out, relations.len());
    for relation in relations {
        text(&mut out, &relation.name.0);
        count(&mut out, relation.columns.len());
        for column in &relation.columns {
            text(&mut out, &column.name.0);
            out.push(match column.kind {
                HostKind::File => 0,
                HostKind::Text => 1,
                HostKind::Number => 2,
            });
        }
        count(&mut out, relation.rows.len());
        for row in &relation.rows {
            count(&mut out, row.0.len());
            for cell in &row.0 {
                match cell {
                    Cell::File(path) => {
                        out.push(0);
                        text(&mut out, path);
                    }
                    Cell::Text(value) => {
                        out.push(1);
                        text(&mut out, value);
                    }
                    Cell::Number(value) => {
                        out.push(2);
                        out.extend_from_slice(&value.0.to_le_bytes());
                    }
                    Cell::Span(value) => {
                        out.push(3);
                        out.extend_from_slice(&value.start.0.to_le_bytes());
                        out.extend_from_slice(&value.end.0.to_le_bytes());
                    }
                }
            }
        }
    }
    crate::domain::keeping::types::DigestInput(out)
}
