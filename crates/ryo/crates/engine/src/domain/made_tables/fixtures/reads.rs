//! What the fill side's cases of the seam share: a table that names a site, one read by key said as text, and what the
//! language was asked.

use crate::domain::code::types::IndexSite;
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{MadeFill, MadeTable};
use crate::domain::made_tables::fixtures::packs::FIRST;
use crate::domain::made_tables::fixtures::packs::{of, operands, pipeline, scan_made, table};
use crate::domain::made_tables::fixtures::{World, framed};
use crate::domain::made_tables::reading::service::rows_at;
use crate::domain::made_tables::sources::service::{handle_of, matched};
use crate::domain::made_tables::types::{KeyFrom, KeyRows};
use crate::domain::steps::stages::types::{FillStage, UseMode};
use crate::domain::steps::types::{MatchColumns, ReadColumn, Register, StepRows};
use ryo_language::domain::facts::types::{CellWord, Wanted};
use ryo_values::domain::plan_parts::types::RepeatedRows;

/// A table whose one pipeline scans table 0 and probes `source` by `columns` with the cell it read: the encoding
/// writes the site of that index.
pub(crate) fn prober(source: StepRows, columns: Vec<ReadColumn>) -> MadeTable {
    let mut all = Vec::from(scan_made(0, 0, 1));
    let values = match columns.len() {
        1 => operands([of(1)]),
        _ => operands([of(1), of(1)]),
    };
    all.push(FillStage::Probe {
        source,
        columns: MatchColumns(columns.into()),
        values,
        range: None,
        mode: UseMode::Present,
        out: Register(2),
    });
    table(
        91,
        1,
        MadeFill::Rows(Box::new([pipeline(all, [of(1)])]), RepeatedRows::Kept),
    )
}

/// The rows an answer holds, as the cells a program binds, and what the read made the engine do.
pub(crate) fn held(world: &World, rows: KeyRows) -> String {
    let cells: Vec<u64> = rows_at(&world.store, rows.rows)
        .iter()
        .map(|row| handle_of(*row, rows.tag).0)
        .collect();
    format!(
        "{cells:?} filled {} indexed {}",
        rows.filled.0, rows.indexed.0
    )
}

/// One read by key through the seam, said as text.
pub(crate) fn asked(
    world: &mut World,
    site: IndexSite,
    key: &[u64],
    from: KeyFrom,
) -> Result<String, FilledError> {
    let key: Vec<CellWord> = key.iter().map(|cell| CellWord(*cell)).collect();
    let rows = {
        let (mut frame, looped) = framed(world);
        matched(&mut frame, looped, site, &key, from)?
    };
    Ok(held(world, rows))
}

/// What the language was asked, as text.
pub(crate) fn told_of(world: &World) -> String {
    let each: Vec<String> = world
        .told
        .asked
        .iter()
        .map(|wanted| match wanted {
            Wanted::Whole => "whole".to_owned(),
            Wanted::Key(column, cell) => format!(
                "{} {}",
                if *column == FIRST { "first" } else { "second" },
                cell.0
            ),
        })
        .collect();
    each.join(", ")
}
