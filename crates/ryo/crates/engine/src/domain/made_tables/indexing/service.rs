//! The services of the `indexing` part: the index of a site built over its source, which is there: a table made, a
//! relation filled whole, the file's nodes. Every source takes the one index; only where a key's cells are read differs.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::service::list;
use crate::domain::code::small_codes::service::node_column_of;
use crate::domain::code::types::{CodeWord, IndexSite, Short, WordSlot};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::{MissingCode, UnbuiltFill};
use crate::domain::indexes::counting::service::{own_rows, place};
use crate::domain::indexes::hashing::service::coded;
use crate::domain::indexes::reading::service::key_cell;
use crate::domain::indexes::service::{based, begin, finish, fixed_keys, key_column, state_of};
use crate::domain::indexes::types::{
    CellOfKey, ColumnView, CountedKey, IndexBuilt, KeyCells, SourceRow,
};
use crate::domain::made_tables::carrying::service::stage_reach;
use crate::domain::made_tables::reading::service::{
    fixed_of, node_names, rows_in, site_at, site_source, view_of,
};
use crate::domain::made_tables::types::{FillFrame, Reached, ReadSource};
use crate::domain::stages::nodes::service::{field_word, kind_word, node_cells, node_tag};
use ryo_language::domain::node_table::service::{field_of, kind_of, parent_of};
use ryo_relations::domain::relations::types::{Count, NodeColumn, Position};

/// A column code as the short a view is asked by.
fn short(code: WordSlot) -> Result<Short, FilledError> {
    u16::try_from(code.0)
        .map(Short)
        .map_err(|_| FilledError::Code {
            named: MissingCode::Column,
        })
}

/// Builds the index a stage asks, when the stage asks one and it is not built for the file: called for the first batch
/// that reaches the stage with a row, its source being there already (`making::service::sources_there`).
///
/// # Errors
///
/// As `index_built`.
pub(crate) fn index_ready(frame: &mut FillFrame<'_>, word: CodeWord) -> Result<(), FilledError> {
    if let Some(Reached::Index(site)) = stage_reach(frame.programs.code, word)?
        && state_of(&frame.store.indexes, site, frame.store.stamp) != IndexBuilt::Built
    {
        let _rows = index_built(frame, site)?;
    }
    Ok(())
}

/// Builds the index of `site` for the file: the rows of its source by its columns, sorted within a key by its order column
/// when it has one. Answers how many rows it indexed. The source is there: over a relation the index is of the whole
/// fill, whose rows are numbered from where it starts.
///
/// # Errors
///
/// The site names a source or a column the code does not hold; or it is an ordered index over the node table, which the
/// fill side does not build yet.
pub(crate) fn index_built(
    frame: &mut FillFrame<'_>,
    site: IndexSite,
) -> Result<Count, FilledError> {
    let programs = frame.programs;
    let code = programs.code;
    let index = site_at(code, site)?;
    let source = site_source(code, index)?;
    let columns = list(&code.pool, index.columns);
    let rows = rows_in(frame.store, frame.nodes, source);
    let count = Count(rows.end.0.saturating_sub(rows.start.0));
    let stamp = frame.store.stamp;
    match source {
        ReadSource::Nodes(_) => {
            if index.ordered != NO_SLOT {
                return Err(FilledError::Unbuilt {
                    unbuilt: UnbuiltFill::OrderedNodes,
                });
            }
            let names = node_names(programs, frame.text, frame.source, frame.file);
            let end = Position(u32::try_from(rows.end.0).unwrap_or(u32::MAX));
            // A key that is one column the node table holds as small numbers needs no cells and no hash of a row.
            let one = match columns {
                [only] if count.0 != 0 => match short(*only) {
                    Ok(code) => node_column_of(code),
                    Err(_) => None,
                },
                _ => None,
            };
            let nodes = frame.nodes;
            let elements = || (0..end.0).map(Position);
            let placed = match one {
                Some(NodeColumn::Id) => {
                    own_rows(
                        &mut frame.store.indexes,
                        site,
                        stamp,
                        node_tag(names.file),
                        count,
                    );
                    true
                }
                Some(NodeColumn::Parent) => {
                    // A parent is an element of the same rows: its id less the file's first id is its place.
                    let key = CountedKey {
                        least: node_tag(names.file),
                        span: Count(count.0.saturating_sub(1)),
                        rows: count,
                    };
                    place(&mut frame.store.indexes, site, stamp, key, || {
                        elements().map(|at| {
                            Count(usize::try_from(parent_of(nodes, at).0).unwrap_or(usize::MAX))
                        })
                    });
                    true
                }
                Some(NodeColumn::Kind) => {
                    let built = coded(
                        &mut frame.store.indexes,
                        site,
                        stamp,
                        count,
                        |code| kind_word(names, code),
                        || elements().map(|at| Count(usize::from(kind_of(nodes, at).0))),
                    );
                    built == KeyCells::Close
                }
                Some(NodeColumn::Field) => {
                    let built = coded(
                        &mut frame.store.indexes,
                        site,
                        stamp,
                        count,
                        |code| field_word(names, code),
                        || elements().map(|at| Count(usize::from(field_of(nodes, at).0))),
                    );
                    built == KeyCells::Close
                }
                Some(
                    NodeColumn::Ordinal
                    | NodeColumn::Start
                    | NodeColumn::End
                    | NodeColumn::Span
                    | NodeColumn::File
                    | NodeColumn::Text,
                )
                | None => false,
            };
            if placed {
                return Ok(count);
            }
            let mut cells = std::mem::take(&mut frame.store.cells);
            begin(&mut frame.store.indexes, count, Count(columns.len()));
            let mut done = Ok(());
            for (at, column) in columns.iter().enumerate() {
                match short(*column).map(node_column_of) {
                    Ok(Some(named)) => {
                        node_cells(
                            frame.nodes,
                            names,
                            frame.words,
                            &mut frame.store.node_texts,
                            named,
                            end,
                            &mut cells,
                        );
                        key_column(
                            &mut frame.store.indexes,
                            Count(at),
                            ColumnView {
                                cells: &cells,
                                first: Count(0),
                                stride: Count(1),
                            },
                        );
                    }
                    Ok(None) | Err(_) => {
                        done = Err(FilledError::Code {
                            named: MissingCode::Column,
                        });
                    }
                }
            }
            frame.store.cells = cells;
            done?;
            finish(&mut frame.store.indexes, site, stamp, None);
        }
        ReadSource::Filled(_) | ReadSource::Made(_) => {
            // The arena is taken while the views read the store's columns.
            let mut arena = std::mem::take(&mut frame.store.indexes);
            // A made table keeps no cell of the file's own column: a key holds the other columns alone.
            let fixed = fixed_of(frame.store, source, columns);
            let held = |at: Count| key_cell(fixed.skipped, at) == CellOfKey::Kept;
            let kept = (0..columns.len()).filter(|at| held(Count(*at))).count();
            begin(&mut arena, count, Count(kept));
            based(
                &mut arena,
                SourceRow(u32::try_from(rows.start.0).unwrap_or(u32::MAX)),
            );
            fixed_keys(&mut arena, fixed);
            let mut done = Ok(());
            let listed = columns
                .iter()
                .enumerate()
                .filter(|(at, _)| held(Count(*at)))
                .map(|(_, column)| column);
            for (at, column) in listed.enumerate() {
                match short(*column) {
                    Ok(place) => {
                        let view = view_of(frame.store, source, place);
                        key_column(
                            &mut arena,
                            Count(at),
                            ColumnView {
                                first: Count(
                                    view.first
                                        .0
                                        .saturating_add(rows.start.0.saturating_mul(view.stride.0)),
                                ),
                                ..view
                            },
                        );
                    }
                    Err(fault) => done = Err(fault),
                }
            }
            if done.is_ok() {
                done = if index.ordered == NO_SLOT {
                    finish(&mut arena, site, stamp, None);
                    Ok(())
                } else {
                    short(index.ordered).map(|place| {
                        finish(
                            &mut arena,
                            site,
                            stamp,
                            Some(view_of(frame.store, source, place)),
                        );
                    })
                };
            }
            frame.store.indexes = arena;
            done?;
        }
        ReadSource::Written(_) => {
            return Err(FilledError::Unbuilt {
                unbuilt: UnbuiltFill::Written,
            });
        }
    }
    Ok(count)
}
