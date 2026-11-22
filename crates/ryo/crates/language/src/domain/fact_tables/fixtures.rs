//! The test support of the `fact_tables` concept: a file's node cells, a row of any relation, a set answered from rows of
//! cells, and the reads a case compares: what a language's contract test and the engine's cases build fact sets with.

use super::reading::service::cells_of;
use super::service::{begun, finished, request_of, row, writer};
use super::types::{FactCells, FactRowList, FactSet, FactTable};
use crate::domain::facts::types::CellWord;
use ryo_relations::domain::relations::service::{columns, kind, node_id};
use ryo_relations::domain::relations::types::{CellKind, Column, FileId, Position, Relation};

/// The file of every case.
pub const FILE: FileId = FileId(3);

/// The cell of node `place` of the file.
#[must_use]
pub fn node(place: u32) -> CellWord {
    CellWord(node_id(FILE, Position(place)).0)
}

/// The cells of a row of `relation`: `place` its node in every node column, `serial` every other cell.
#[must_use]
pub fn row_of(relation: Relation, place: u32, serial: u64) -> Vec<CellWord> {
    columns(relation)
        .into_iter()
        .map(|column| match kind(column) {
            CellKind::Node => node(place),
            CellKind::Symbol
            | CellKind::File
            | CellKind::Span
            | CellKind::Number
            | CellKind::Flag => CellWord(serial),
        })
        .collect()
}

/// A set that answers a request of the relations of `answered`, each with its rows, written in the order given.
#[must_use]
pub fn answered(answered: &[(Relation, Vec<Vec<CellWord>>)]) -> FactSet {
    let relations: Vec<Relation> = answered.iter().map(|(relation, _)| *relation).collect();
    let mut set = FactSet::default();
    begun(&mut set, &request_of(&relations));
    for (relation, rows) in answered {
        let mut writing = writer(&mut set, *relation);
        for cells in rows {
            row(&mut writing, FactCells(cells));
        }
        finished(writing);
    }
    set
}

/// How many rows a lookup found.
#[must_use]
pub fn length(rows: FactRowList<'_>) -> u64 {
    u64::try_from(rows.into_iter().count()).unwrap_or(u64::MAX)
}

/// The cells of `column` over the rows a lookup found, in its order.
#[must_use]
pub fn read(table: &FactTable, column: Column, rows: FactRowList<'_>) -> Vec<CellWord> {
    let cells = cells_of(table, column);
    rows.into_iter()
        .filter_map(|at| {
            cells
                .0
                .get(usize::try_from(at.0).unwrap_or(usize::MAX))
                .copied()
        })
        .collect()
}
