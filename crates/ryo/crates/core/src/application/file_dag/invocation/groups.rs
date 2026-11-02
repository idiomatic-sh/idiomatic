//! Existing grouping semantic accumulator; input driving remains in the common engine.
use super::*;
use crate::domain::{
    table_cells::types::RowIndex,
    table_groups::{
        service::{count_groups, digest_groups},
        types::{CountRoom, Hashes},
    },
    table_model::{service::push_row, types::On},
};
fn materialized(input: BorrowedRows<'_>) -> Table {
    let mut table = Table {
        kinds: input.kinds().to_vec(),
        ..Table::default()
    };
    let mut row = Vec::new();
    for at in 0..input.count() {
        if let Some(row) = input.read(RowIndex(at), &mut row) {
            push_row(&mut table, row);
        }
    }
    table
}
pub(super) fn groups(
    input: BorrowedRows<'_>,
    keys: &[ColumnIndex],
    distinct: Option<ColumnIndex>,
    digest: Option<ColumnIndex>,
    context: &OperatorContext<'_>,
) -> Result<Vec<Group>, OperatingError> {
    let table = materialized(input);
    let mut groups = Vec::new();
    let mut order = KeyIndex::default();
    let mut values = Vec::new();
    let mut room = CountRoom {
        order: &mut order,
        groups: &mut groups,
        values: &mut values,
    };
    if let Some(over) = digest {
        let texts = Texts {
            symbols: context.symbols.as_ref(),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
        };
        digest_groups(
            &table,
            keys,
            over,
            &texts,
            &mut room,
            &mut Hashes::default(),
        )?;
    } else {
        count_groups(&table, On::Columns(keys), distinct, &mut room)?;
    }
    Ok(groups)
}
