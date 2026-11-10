//! The engine chooses the physical probe; this kernel owns ordered Join row semantics.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::{
    Advance, JoinDriver, JoinRow, JoinShape, PositionSlots, PositionTarget,
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};

/// Ordered Join meaning over the engine-selected borrowed left representation.
pub(crate) fn join_row(
    left: JoinRow<'_>,
    left_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    driver: &mut dyn JoinDriver,
    key: &mut Vec<Value>,
    target: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    let read_left = |column: ColumnIndex| match &left {
        JoinRow::Values(row) => {
            let Ok(at) = usize::try_from(column.0) else {
                return None;
            };
            row.get(at).copied()
        }
        JoinRow::Columns(row) => row.cell(column),
    };
    key.clear();
    for column in left_on {
        key.push(read_left(*column).ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?);
    }
    while driver
        .next(key)
        .map_err(|_| PortableOperatorFault::Interrupted)?
        == Advance::Admitted
    {
        target.clear();
        for column in output {
            let value = match column.side {
                Side::Left => read_left(column.column),
                Side::Right => driver.right_cell(
                    match view.get(usize::try_from(column.column.0).unwrap_or(usize::MAX)) {
                        Some(physical) => *physical,
                        None => column.column,
                    },
                ),
            }
            .ok_or(PortableOperatorFault::NoSuchColumn {
                index: column.column,
            })?;
            target.push(value);
        }
        driver
            .emit(target)
            .map_err(|_| PortableOperatorFault::Interrupted)?;
    }
    Ok(())
}

/// A lowered row's schema already maps logical columns onto these slots. Keep
/// them deferred: only demanded key/filter/output reads may decode cells.
pub(crate) fn positions(
    left: PositionSlots<'_>,
    driver: &mut dyn crate::domain::equality_join::types::PositionJoinDriver,
    target: PositionTarget<'_>,
) -> Result<(), PortableOperatorFault> {
    let (left, target) = (left.0, target.0);
    while driver
        .next()
        .map_err(|_| PortableOperatorFault::Interrupted)?
        == Advance::Admitted
    {
        target.extend_from_slice(left);
        target.extend_from_slice(driver.right_positions().0);
    }
    Ok(())
}

/// One output row of a join from its left row and a right row of logical columns.
fn joined(
    output: &[JoinColumn],
    left: &[Value],
    right: &[Value],
    answer: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    answer.clear();
    for column in output {
        answer.push(match column.side {
            Side::Left => ryo_values::domain::rows::service::column_in(left, column.column)
                .copied()
                .ok_or(
                    crate::domain::contract::types::PortableOperatorFault::NoSuchColumn {
                        index: column.column,
                    },
                )?,
            Side::Right => ryo_values::domain::rows::service::column_in(right, column.column)
                .copied()
                .ok_or(
                    crate::domain::contract::types::PortableOperatorFault::NoSuchColumn {
                        index: column.column,
                    },
                )?,
        });
    }
    Ok(())
}

/// Whether a right row's key columns hold the left row's key: the row is kept when they do.
fn keyed(
    right_on: &[ColumnIndex],
    key: &[Value],
    right: &[Value],
) -> crate::domain::contract::types::PredicateOutcome {
    if right_on
        .iter()
        .zip(key)
        .all(|(column, value)| right.get(usize::from(*column)) == Some(value))
    {
        crate::domain::contract::types::PredicateOutcome::Holds
    } else {
        crate::domain::contract::types::PredicateOutcome::Fails
    }
}

/// One logical column of a closure row: a column of the partition's key, the source or the vertex it reaches.
fn closure_cell(
    key: &[Value],
    column: ColumnIndex,
    source: Value,
    reached: Value,
) -> Result<Value, PortableOperatorFault> {
    let at = usize::try_from(column.0).map_err(|_| PortableOperatorFault::ShapeMismatch)?;
    if let Some(each) = key.get(at) {
        Ok(*each)
    } else if at == key.len() {
        Ok(source)
    } else if at == key.len() + 1 {
        Ok(reached)
    } else {
        Err(PortableOperatorFault::ShapeMismatch)
    }
}

/// Joins a left row with every vertex a closure reaches from the sources its key binds; the projection of a
/// logical closure has set semantics, so each right row is joined the first time the invocation sees it.
fn reach_join(
    left: &[Value],
    shape: JoinShape<'_>,
    reach: &ryo_values::domain::plan_parts::types::ReachLookup,
    cached: Option<crate::domain::equality_join::types::CachedReach<'_>>,
    driver: &mut dyn crate::domain::equality_join::types::JoinEvaluation,
    scratch: &mut crate::domain::contract::types::RowScratch,
) -> Result<(), PortableOperatorFault> {
    use crate::domain::closure::types::PartitionPlace;
    use crate::domain::contract::types::FirstOccurrence;
    use crate::domain::contract::types::VertexPlace;
    let crate::domain::contract::types::RowScratch {
        row: right,
        other: answer,
        key,
        partition: partition_key,
        ..
    } = scratch;
    key.clear();
    for column in shape.left_on {
        key.push(
            ryo_values::domain::rows::service::column_in(left, *column)
                .copied()
                .ok_or(
                    crate::domain::contract::types::PortableOperatorFault::NoSuchColumn {
                        index: *column,
                    },
                )?,
        );
    }
    if let Some(graph) = cached {
        for destination in graph.destinations {
            right.clear();
            for column in &reach.columns {
                let at =
                    usize::try_from(column.0).map_err(|_| PortableOperatorFault::ShapeMismatch)?;
                right.push(if at < reach.keys.len() + 1 {
                    *graph
                        .request
                        .get(at)
                        .ok_or(PortableOperatorFault::ShapeMismatch)?
                } else if at == reach.keys.len() + 1 {
                    *destination
                } else {
                    return Err(PortableOperatorFault::ShapeMismatch);
                });
            }
            if keyed(shape.right_on, key, right)
                == crate::domain::contract::types::PredicateOutcome::Fails
            {
                continue;
            }
            if driver.first_time(right)? != FirstOccurrence::First {
                continue;
            }
            joined(shape.output, left, right, answer)?;
            driver.emit(answer)?;
        }
        return Ok(());
    }
    let partitions = driver.closure().partitions()?;
    for place in (0..partitions.0).map(PartitionPlace) {
        driver.closure().partition_key(place, partition_key)?;
        let sources = driver.closure().sources(place);
        for at in (0..sources.0).map(VertexPlace) {
            let source = driver
                .closure()
                .source(place, at)?
                .ok_or(PortableOperatorFault::ShapeMismatch)?;
            // Bind partition and source before the walk whenever the logical key names them.
            if !shape
                .right_on
                .iter()
                .zip(key.iter())
                .all(|(column, value)| {
                    let Some(logical) = reach.columns.get(usize::from(*column)) else {
                        return false;
                    };
                    let at = usize::from(*logical);
                    if at < partition_key.len() {
                        partition_key.get(at) == Some(value)
                    } else if at == partition_key.len() {
                        source == *value
                    } else {
                        true
                    }
                })
            {
                continue;
            }
            let reached = driver.closure().reach(place, source)?;
            for vertex in (0..reached.0).map(VertexPlace) {
                let vertex = driver
                    .closure()
                    .reached(vertex)
                    .ok_or(PortableOperatorFault::ShapeMismatch)?;
                right.clear();
                for column in &reach.columns {
                    right.push(closure_cell(partition_key, *column, source, vertex)?);
                }
                if keyed(shape.right_on, key, right)
                    == crate::domain::contract::types::PredicateOutcome::Fails
                {
                    continue;
                }
                if driver.first_time(right)? != FirstOccurrence::First {
                    continue;
                }
                joined(shape.output, left, right, answer)?;
                driver.emit(answer)?;
            }
        }
    }
    Ok(())
}

/// Joins a scheduled left row with its right candidates: an equality join over the engine's candidate
/// source, or a reachability join over a closure.
pub(crate) fn evaluate(
    left: &[Value],
    shape: JoinShape<'_>,
    cached: Option<crate::domain::equality_join::types::CachedReach<'_>>,
    driver: &mut dyn crate::domain::equality_join::types::JoinEvaluation,
    scratch: &mut crate::domain::contract::types::RowScratch,
) -> Result<(), PortableOperatorFault> {
    if let Some(reach) = shape.reach {
        return reach_join(left, shape, reach, cached, driver, scratch);
    }
    // Indexed candidates already answer the requested logical view; native tree candidates answer
    // physical tree columns, which the declared view remaps.
    let view = match driver.right_columns() {
        crate::domain::contract::types::RightColumns::Physical => shape.view,
        crate::domain::contract::types::RightColumns::Logical => &[],
    };
    join_row(
        JoinRow::Values(left),
        shape.left_on,
        shape.output,
        view,
        driver.join(),
        &mut scratch.key,
        &mut scratch.other,
    )
}
