//! The `graph` service: free functions named by verb over the domain's types, with no IO and no ports.
use crate::domain::operator::contract::driver::service::hold;
use crate::domain::operator::contract::driver::service::interrupted;

use super::super::driver::types::LeaseDriver;
use super::super::primitive::types::PreparedInputs;
use super::super::types::{ExaminedCount, ExpandedCount};
use super::super::views::types::PreparedAdjacency;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_reach::types::{Edges, OrderedVisit};
use ryo_operators::domain::closure::types::PartitionPlace;
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::contract::types::{RowRead, VertexPlace};
use ryo_operators::domain::cycles::types::{CycleDriver, EdgePlace, NodeNumber};
use ryo_operators::domain::ordered_dfs::types::OrderedEdge;
use ryo_operators::domain::ordered_match::types::OrderedMatchDriver;
use ryo_relations::domain::relations::types::{Count, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// The walked edges of the depth-first graph: its edge keys but the last, toward the last.
pub(crate) fn dfs_edges<'f>(driver: &LeaseDriver<'_, 'f>) -> Result<Edges<'f>, OperatingError> {
    let graph = driver.graph.dfs.ok_or(OperatingError::WrongBinding)?;
    let columns = graph
        .edge_columns
        .get(
            ..graph
                .edge_columns
                .len()
                .checked_sub(1)
                .ok_or(OperatingError::ShapeMismatch)?,
        )
        .ok_or(OperatingError::ShapeMismatch)?;
    let target = *graph
        .edge_columns
        .last()
        .ok_or(OperatingError::ShapeMismatch)?;
    Ok(Edges {
        table: &graph.edges,
        order: &graph.edge_index,
        from: columns,
        to: target,
    })
}

pub(crate) fn reach_partition<'f>(
    driver: &mut LeaseDriver<'_, 'f>,
    partition: PartitionPlace,
    source: Value,
) -> Result<Count, OperatingError> {
    let partition = crate::domain::operator::contract::views::service::get(
        driver.graph.closure,
        super::super::views::types::PartitionIndex(partition.0),
    )?;
    let walk = driver
        .lease
        .workspace
        .closure_walk
        .as_mut()
        .ok_or(OperatingError::WrongBinding)?;
    crate::domain::table_reach::service::reach_all(
        &Edges {
            table: &partition.table,
            order: &partition.index,
            from: &[ColumnIndex(0)],
            to: ColumnIndex(1),
        },
        source,
        walk,
    );
    Ok(Count(walk.reached.len()))
}

pub(crate) fn next_visit<'f>(
    driver: &mut LeaseDriver<'_, 'f>,
    partition: &[Value],
) -> Result<Option<OrderedEdge>, OperatingError> {
    let graph = driver.graph.dfs.ok_or(OperatingError::WrongBinding)?;
    let edges = dfs_edges(driver)?;
    let work = driver
        .lease
        .workspace
        .dfs
        .as_mut()
        .ok_or(OperatingError::WrongBinding)?;
    let visit = crate::domain::table_reach::service::next_ordered(
        &edges,
        graph,
        partition,
        &mut work.walk,
    )?;
    if visit.is_none() {
        // The walk from this root is over: what it expanded and examined counts for the invocation.
        work.expanded.0 += work.walk.expanded.0;
        work.examined.0 += work.walk.examined.0;
    }
    Ok(visit.map(|OrderedVisit { edge, ordinal }| OrderedEdge { edge, ordinal }))
}

pub(crate) fn adjacency_edge(
    adjacency: &PreparedAdjacency<'_>,
    node: NodeNumber,
    place: EdgePlace,
) -> Option<NodeNumber> {
    let starts = &adjacency.0.starts.0;
    let at = starts.get(node.0)?.checked_add(place.0)?;
    (at < *starts.get(node.0 + 1)?)
        .then(|| adjacency.0.leaving.0.get(at).copied().map(NodeNumber))
        .flatten()
}

pub(crate) fn partitions(driver: &mut LeaseDriver<'_, '_>) -> Result<Count, Fault> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let closure = driver.resources.closure();
    let closure = hold(driver, closure)?;
    driver.graph.closure = closure;
    Ok(Count(
        crate::domain::operator::contract::views::service::len(closure).0,
    ))
}

pub(crate) fn partition_key(
    driver: &mut LeaseDriver<'_, '_>,
    partition: PartitionPlace,
    key: &mut Vec<Value>,
) -> Result<(), Fault> {
    let found = crate::domain::operator::contract::views::service::get(
        driver.graph.closure,
        super::super::views::types::PartitionIndex(partition.0),
    )
    .map(|partition| {
        key.clear();
        key.extend_from_slice(&partition.key);
    });
    hold(driver, found)
}

pub(crate) fn sources(driver: &LeaseDriver<'_, '_>, partition: PartitionPlace) -> Count {
    Count(
        crate::domain::operator::contract::views::service::get(
            driver.graph.closure,
            super::super::views::types::PartitionIndex(partition.0),
        )
        .map_or(0, |partition| partition.sources.len()),
    )
}

pub(crate) fn source(
    driver: &mut LeaseDriver<'_, '_>,
    partition: PartitionPlace,
    at: VertexPlace,
) -> Result<Option<Value>, Fault> {
    let found = crate::domain::operator::contract::views::service::get(
        driver.graph.closure,
        super::super::views::types::PartitionIndex(partition.0),
    )
    .map(|partition| partition.sources.get(at.0).copied());
    hold(driver, found)
}

pub(crate) fn reach(
    driver: &mut LeaseDriver<'_, '_>,
    partition: PartitionPlace,
    source: Value,
) -> Result<Count, Fault> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let reached = reach_partition(driver, partition, source);
    hold(driver, reached)
}

pub(crate) fn reached(driver: &LeaseDriver<'_, '_>, at: VertexPlace) -> Option<Value> {
    driver
        .lease
        .workspace
        .closure_walk
        .as_ref()?
        .reached
        .get(at.0)
        .copied()
}

pub(crate) fn roots(driver: &mut LeaseDriver<'_, '_>) -> Result<RowCount, Fault> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let graph = driver
        .resources
        .dfs()
        .and_then(|graph| graph.ok_or(OperatingError::WrongBinding));
    let graph = hold(driver, graph)?;
    driver.graph.dfs = Some(graph);
    let checked = dfs_edges(driver).map(|_| ()).and_then(|()| {
        let work = driver
            .lease
            .workspace
            .dfs
            .as_mut()
            .ok_or(OperatingError::WrongBinding)?;
        work.expanded = ExpandedCount::default();
        work.examined = ExaminedCount::default();
        Ok(())
    });
    hold(driver, checked)?;
    Ok(RowCount(graph.root_index.sorted.len()))
}

pub(crate) fn root(driver: &LeaseDriver<'_, '_>, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
    let Some(graph) = driver.graph.dfs else {
        return RowRead::Missing;
    };
    let Some(root) = graph
        .root_index
        .sorted
        .get(at.0)
        .and_then(|sorted| ryo_values::domain::rows::service::row_in(&graph.roots, *sorted))
    else {
        return RowRead::Missing;
    };
    row.clear();
    row.extend_from_slice(root);
    RowRead::Read
}

pub(crate) fn root_columns(driver: &LeaseDriver<'_, '_>, columns: &mut Vec<ColumnIndex>) {
    columns.clear();
    if let Some(graph) = driver.graph.dfs {
        columns.extend_from_slice(&graph.root_columns);
    }
}

pub(crate) fn begin(
    driver: &mut LeaseDriver<'_, '_>,
    partition: &[Value],
    root: Value,
) -> Result<(), Fault> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let begun = driver
        .graph
        .dfs
        .ok_or(OperatingError::WrongBinding)
        .and_then(|graph| {
            let edges = dfs_edges(driver)?;
            let work = driver
                .lease
                .workspace
                .dfs
                .as_mut()
                .ok_or(OperatingError::WrongBinding)?;
            crate::domain::table_reach::service::begin_ordered(
                &edges,
                graph,
                partition,
                root,
                &mut work.walk,
            );
            Ok(())
        });
    hold(driver, begun)
}

pub(crate) fn next(
    driver: &mut LeaseDriver<'_, '_>,
    partition: &[Value],
) -> Result<Option<OrderedEdge>, Fault> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let visit = next_visit(driver, partition);
    hold(driver, visit)
}

pub(crate) fn edge(driver: &LeaseDriver<'_, '_>, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
    let Some(edge) = driver
        .graph
        .dfs
        .and_then(|graph| ryo_values::domain::rows::service::row_in(&graph.edges, at))
    else {
        return RowRead::Missing;
    };
    row.clear();
    row.extend_from_slice(edge);
    RowRead::Read
}

pub(crate) fn cycles<'r>(
    driver: &'r mut LeaseDriver<'_, '_>,
) -> Result<&'r dyn CycleDriver, Fault> {
    let graph = driver
        .resources
        .cycles()
        .and_then(|graph| graph.ok_or(OperatingError::WrongBinding));
    let graph = hold(driver, graph)?;
    driver.graph.cycles = Some(PreparedAdjacency(graph));
    driver
        .graph
        .cycles
        .as_ref()
        .map(|graph| -> &dyn CycleDriver { graph })
        .ok_or(Fault::WrongBinding)
}

pub(crate) fn ordered_match<'r>(
    driver: &'r mut LeaseDriver<'_, '_>,
) -> Result<&'r mut dyn OrderedMatchDriver, Fault> {
    let checked = match driver.inputs {
        PreparedInputs::Views(inputs) => inputs
            .first()
            .ok_or(OperatingError::MissingInput { position: 1 })
            .and_then(|_| {
                inputs
                    .get(1)
                    .ok_or(OperatingError::MissingInput { position: 2 })
            })
            .and_then(|_| driver.resources.ordered())
            .and_then(|ordered| ordered.ok_or(OperatingError::WrongBinding)),
        PreparedInputs::Row { .. } | PreparedInputs::RequestSource(_) => {
            Err(OperatingError::WrongOperation)
        }
    };
    driver.graph.ordered = Some(hold(driver, checked)?);
    Ok(driver)
}

pub(crate) fn compare_keys(
    driver: &LeaseDriver<'_, '_>,
    left: RowIndex,
    right: RowIndex,
) -> std::cmp::Ordering {
    driver
        .graph
        .ordered
        .map_or(std::cmp::Ordering::Equal, |ordered| {
            ordered.left[left.0].keys.cmp(&ordered.right[right.0].keys)
        })
}

pub(crate) fn right_order_before_or_equal(
    driver: &LeaseDriver<'_, '_>,
    left: RowIndex,
    right: RowIndex,
) -> ryo_operators::domain::ordered_match::types::KeyOrder {
    use ryo_operators::domain::ordered_match::types::KeyOrder;
    match driver.graph.ordered {
        Some(ordered) if ordered.right[right.0].order <= ordered.left[left.0].order => {
            KeyOrder::BeforeOrEqual
        }
        Some(_) | None => KeyOrder::After,
    }
}

pub(crate) fn visit_pair(
    driver: &mut LeaseDriver<'_, '_>,
    left: RowIndex,
    right: RowIndex,
    project: &mut dyn FnMut(&[Value], &[Value]) -> Result<(), Fault>,
) -> Result<(), Fault> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(Fault::Interrupted);
    }
    let (Some(ordered), PreparedInputs::Views(inputs)) = (driver.graph.ordered, driver.inputs)
    else {
        return hold(driver, Err(OperatingError::WrongBinding));
    };
    let (Some(left_rows), Some(right_rows)) = (inputs.first(), inputs.get(1)) else {
        return hold(driver, Err(OperatingError::ShapeMismatch));
    };
    let (left_buffer, right_buffer) = (&mut driver.left_buffer, &mut driver.right_buffer);
    let pair = left_rows
        .read(ordered.left[left.0].row, left_buffer)
        .ok_or(OperatingError::ShapeMismatch)
        .and_then(|left| {
            right_rows
                .read(ordered.right[right.0].row, right_buffer)
                .ok_or(OperatingError::ShapeMismatch)
                .map(|right| project(left, right))
        });
    hold(driver, pair)?
}
