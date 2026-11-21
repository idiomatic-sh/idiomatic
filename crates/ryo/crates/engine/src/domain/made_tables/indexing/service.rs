//! The services of the `indexing` part: the index of a site built over its source, which is there: a table made, a
//! relation filled whole, the file's nodes. Every source takes the one index; only where a key's cells are read differs.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::service::list;
use crate::domain::code::small_codes::service::node_column_of;
use crate::domain::code::types::{IndexSite, Short, WordSlot};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::{MissingCode, UnbuiltFill};
use crate::domain::indexes::service::{based, begin, finish, key_column};
use crate::domain::indexes::types::{ColumnView, SourceRow};
use crate::domain::made_tables::reading::service::{
    node_names, rows_in, site_at, site_source, view_of,
};
use crate::domain::made_tables::types::{FillFrame, ReadSource};
use crate::domain::stages::nodes::service::node_cells;
use ryo_relations::domain::relations::types::{Count, Position};

/// A column code as the short a view is asked by.
fn short(code: WordSlot) -> Result<Short, FilledError> {
    u16::try_from(code.0)
        .map(Short)
        .map_err(|_| FilledError::Code {
            named: MissingCode::Column,
        })
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
            let names = node_names(programs, frame.text, frame.file);
            let end = Position(u32::try_from(rows.end.0).unwrap_or(u32::MAX));
            let mut cells = std::mem::take(&mut frame.store.cells);
            begin(&mut frame.store.indexes, count, Count(columns.len()));
            let mut done = Ok(());
            for (at, column) in columns.iter().enumerate() {
                match short(*column).map(node_column_of) {
                    Ok(Some(named)) => {
                        node_cells(frame.nodes, names, frame.words, named, end, &mut cells);
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
            begin(&mut arena, count, Count(columns.len()));
            based(
                &mut arena,
                SourceRow(u32::try_from(rows.start.0).unwrap_or(u32::MAX)),
            );
            let mut done = Ok(());
            for (at, column) in columns.iter().enumerate() {
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
