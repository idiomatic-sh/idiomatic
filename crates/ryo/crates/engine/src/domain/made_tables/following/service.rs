//! The services of the `following` part: the `Graph(FixedPoints)` fill. The edges are gathered through their pipelines
//! and indexed by the keys (the partition; none: one graph of the file). For each key the entities of its edges are
//! numbered in the order of their cells, the operators crate's meaning (`follow::service::fixed_points_visited`) answers
//! each pair of a source and a fixed point it reaches, and a row of the table is the key, the source, the fixed point.

use crate::domain::code::records::types::EdgeColumns;
use crate::domain::code::types::IndexSite;
use crate::domain::indexes::reading::service::{cell_at, read, rows_of, runs};
use crate::domain::indexes::types::SourceRow;
use crate::domain::made_tables::columns::types::{ColumnIs, RowColumn};
use crate::domain::made_tables::following::types::FollowRoom;
use crate::domain::made_tables::service::{column, file_columns, file_of, grouped, place_of, row};
use crate::domain::made_tables::types::{GroupRoom, TableRows};
use ryo_language::domain::facts::types::CellWord;
use ryo_operators::domain::follow::service::fixed_points_visited;
use ryo_operators::domain::follow::types::{Entity, GroupPointers, Pointer};
use ryo_relations::domain::relations::types::Count;

/// The place a row's number names.
fn place(numbered: SourceRow) -> Count {
    Count(usize::try_from(numbered.0).unwrap_or(usize::MAX))
}

/// The `Graph(FixedPoints)` fill: for each key of the gathered `input` (the keys are `room.keys`), each pair of a source
/// and a fixed point it reaches over the key's edges, each pair once: the key's cells, the source, the fixed point.
/// Answers how many times a walk back read an edge: the fill's work past its input's rows.
pub(crate) fn follow_groups(
    input: &TableRows,
    columns: EdgeColumns,
    room: &mut GroupRoom,
    out: &mut TableRows,
) -> Count {
    grouped(input, room, None);
    let GroupRoom {
        index,
        keys,
        following,
        ..
    } = room;
    // The table's row is the keys, then the source and the fixed point: a key that is the file's own column in the
    // input is the file's in the table.
    let key_at = |at: Count| {
        keys.get(at.0)
            .map(|key| Count(usize::try_from(key.0).unwrap_or(usize::MAX)))
    };
    file_columns(out, file_of(input), |at| match key_at(at) {
        Some(key) if place_of(input, key) == RowColumn::File => ColumnIs::TheFile,
        Some(_) | None => ColumnIs::Written,
    });
    let FollowRoom {
        entities,
        edges,
        pairs,
    } = following;
    let sources = column(input, Count(usize::from(columns.source.0)));
    let destinations = column(input, Count(usize::from(columns.destination.0)));
    let index = read(index, IndexSite(0));
    let mut visits = Count(0);
    for run in runs(index) {
        let held = rows_of(index, run);
        let Some(first) = held.first() else {
            continue;
        };
        entities.clear();
        for each in held {
            entities.push(cell_at(sources, place(*each)));
            entities.push(cell_at(destinations, place(*each)));
        }
        entities.sort_unstable();
        entities.dedup();
        let numbered = |cell: CellWord| {
            Entity(
                u32::try_from(entities.partition_point(|other| *other < cell)).unwrap_or(u32::MAX),
            )
        };
        edges.clear();
        edges.extend(held.iter().map(|each| Pointer {
            source: numbered(cell_at(sources, place(*each))),
            destination: numbered(cell_at(destinations, place(*each))),
        }));
        edges.sort_unstable();
        pairs.clear();
        let walked = fixed_points_visited(GroupPointers(edges), pairs);
        visits = Count(visits.0.saturating_add(walked.0));
        let key_row = row(input, place(*first));
        for pair in pairs.iter() {
            for key in keys.iter() {
                if let RowColumn::Kept(at) =
                    place_of(input, Count(usize::try_from(key.0).unwrap_or(usize::MAX)))
                {
                    out.cells
                        .push(key_row.get(at.0).copied().unwrap_or(CellWord(0)));
                }
            }
            for end in [pair.source, pair.destination] {
                let cell = entities.get(usize::try_from(end.0).unwrap_or(usize::MAX));
                out.cells.push(cell.copied().unwrap_or(CellWord(0)));
            }
            out.count = Count(out.count.0.saturating_add(1));
        }
    }
    visits
}
