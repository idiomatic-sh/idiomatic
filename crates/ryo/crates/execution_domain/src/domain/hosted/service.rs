//! The `hosted` service: free functions over the relations the host declares, with no IO and no ports.

use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_relations::domain::relations::types::{CellKind, FileId, Value};
use ryo_relations::domain::terms::types::NameText;

use super::types::NumberedFiles;
use {ryo_values::domain::rows::service::push_row, ryo_values::domain::rows::service::reset};

use ryo_values::domain::hosted::service::kind_of;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::row_texts::types::Cell;


/// Materialize a declared request source in the query's derived-name pool.
/// Source-name interning remains unavailable to semantic operators.
pub fn table_of_query(
    relation: &HostRelation,
    files: NumberedFiles<'_>,
    symbols: &mut QuerySymbols<'_>,
    phase: crate::domain::operating::types::AfterTheFiles,
) -> Table {
    table_with(relation, files, |text| {
        let symbol = query_intern_derived(symbols, ByteSlice(&[]), text);
        crate::domain::operating::service::minted(symbols.as_ref(), symbol, phase)
    })
}

fn table_with(
    relation: &HostRelation,
    files: NumberedFiles<'_>,
    mut text_value: impl FnMut(NameText<'_>) -> Value,
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
                Cell::Text(text) => text_value(NameText(text)),
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
