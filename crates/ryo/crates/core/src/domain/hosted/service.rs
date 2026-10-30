//! The `hosted` service: free functions over the relations the host declares, with no IO and no ports.

use ryo_language::domain::relations::types::{CellKind, FileId, Value};
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::terms::types::NameText;

use super::constants::tab;
use super::types::NumberedFiles;
use crate::domain::keeping::types::HostedText;
use crate::domain::table_model::service::{push_row, reset};
use crate::domain::table_model::types::Table;
use ryo_values::domain::hosted::service::kind_of;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::records::types::Cell;

/// The relations as lines of text for the digest of a configuration: another set of relations or rows is another configuration.
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
                Cell::Text(text) => {
                    Value::Symbol(intern_derived(symbols, ByteSlice(&[]), NameText(text)))
                }
                Cell::Number(number) => Value::Number(*number),
                // No column the host declares holds a span: the SDK refuses one, and a row that still has one is no row.
                Cell::Span(_) => continue 'rows,
            });
        }
        push_row(&mut table, &cells);
    }
    table
}
