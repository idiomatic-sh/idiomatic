//! Each distinct root is visited once; each edge it reaches is one row, with the ordinal of the visit.
use super::types::DfsShape;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::contract::types::RowRead;
use crate::domain::contract::types::RowScratch;
use crate::domain::ordered_dfs::types::DfsEvaluation;
use crate::domain::ordered_dfs::types::OrderedEdge;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::rows::types::RowIndex;

/// Visits the engine's prepared graph from each distinct root row, in the roots' sorted order, and emits
/// the root's keys and root, then each reached edge's source, destination and edge, and the ordinal.
pub(crate) fn evaluate(
    shape: DfsShape<'_>,
    driver: &mut dyn DfsEvaluation,
    scratch: &mut RowScratch,
) -> Result<(), PortableOperatorFault> {
    let RowScratch {
        row: joined,
        other: root_row,
        captured: edge_row,
        key,
        partition: prefix,
        columns,
        ..
    } = scratch;
    let roots = driver.dfs().roots()?;
    driver.dfs().root_columns(columns);
    let canonical_root = *columns.last().ok_or(PortableOperatorFault::ShapeMismatch)?;
    let mut begin = 0;
    while begin < roots.0 {
        if driver.dfs().root(RowIndex(begin), root_row) != RowRead::Read {
            return Err(PortableOperatorFault::ShapeMismatch);
        }
        key.clear();
        for column in columns.iter().copied() {
            key.push(
                ryo_values::domain::rows::service::column_in(root_row, column)
                    .copied()
                    .ok_or(
                        crate::domain::contract::types::PortableOperatorFault::NoSuchColumn {
                            index: column,
                        },
                    )?,
            );
        }
        prefix.clear();
        for column in shape.root_keys.iter().copied().chain([shape.root]) {
            prefix.push(
                ryo_values::domain::rows::service::column_in(root_row, column)
                    .copied()
                    .ok_or(
                        crate::domain::contract::types::PortableOperatorFault::NoSuchColumn {
                            index: column,
                        },
                    )?,
            );
        }
        let partition = key
            .get(..shape.root_keys.len())
            .ok_or(PortableOperatorFault::ShapeMismatch)?;
        driver.dfs().begin(
            partition,
            ryo_values::domain::rows::service::column_in(root_row, canonical_root)
                .copied()
                .ok_or(
                    crate::domain::contract::types::PortableOperatorFault::NoSuchColumn {
                        index: canonical_root,
                    },
                )?,
        )?;
        while let Some(OrderedEdge { edge, ordinal }) = driver.dfs().next(partition)? {
            if driver.dfs().edge(edge, edge_row) != RowRead::Read {
                return Err(PortableOperatorFault::ShapeMismatch);
            }
            joined.clear();
            joined.extend_from_slice(prefix);
            for column in shape.edge_columns {
                joined.push(
                    ryo_values::domain::rows::service::column_in(edge_row, column)
                        .copied()
                        .ok_or(
                            crate::domain::contract::types::PortableOperatorFault::NoSuchColumn {
                                index: column,
                            },
                        )?,
                );
            }
            joined.push(Value::Number(ordinal));
            driver.emit(joined)?;
        }
        begin += 1;
        while begin < roots.0 {
            if driver.dfs().root(RowIndex(begin), root_row) != RowRead::Read {
                return Err(PortableOperatorFault::ShapeMismatch);
            }
            if !columns.iter().zip(key.iter()).all(|(column, value)| {
                root_row.get(usize::from(*column)) == Some(value)
            }) {
                break;
            }
            begin += 1;
        }
    }
    Ok(())
}
