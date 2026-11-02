//! Preparation and generation-local acquisition of complete typed compact adjacency.

use super::types::{CompactGraph, GraphIdentity};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::service::{cell_at, check_columns, count_of, kinds_of};
use crate::domain::table_model::types::Rows;
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

/// Prove an eligible physical population and compose selected columns before sharing graph data.
pub(crate) fn source(
    input: ryo_values::domain::plan_parts::types::Input,
    rows: &Rows<'_>,
    endpoints: [ColumnIndex; 2],
    keys: &[ColumnIndex],
    eligible: &[ryo_language::domain::relations::types::Relation],
) -> Option<super::types::GraphSource> {
    let (population, columns) = population(input, rows, eligible, false)?;
    let mapped = |column: ColumnIndex| columns.get(column.0 as usize).copied();
    let first = mapped(endpoints[0])?;
    let second = mapped(endpoints[1])?;
    let forward = first <= second;
    Some(super::types::GraphSource {
        identity: GraphIdentity {
            population,
            endpoints: if forward {
                [first, second]
            } else {
                [second, first]
            },
            keys: keys
                .iter()
                .map(|column| mapped(*column))
                .collect::<Option<_>>()?,
        },
        endpoints: if forward {
            endpoints
        } else {
            [endpoints[1], endpoints[0]]
        },
        orientation: if forward {
            super::types::Orientation::Forward
        } else {
            super::types::Orientation::Reverse
        },
    })
}

/// Exact domain provenance when stable; computed domains remain invocation-local.
pub(crate) fn domain_identity(
    input: ryo_values::domain::plan_parts::types::Input,
    rows: &Rows<'_>,
    column: ColumnIndex,
    eligible: &[ryo_language::domain::relations::types::Relation],
) -> Option<super::types::DomainIdentity> {
    let (population, columns) = population(input, rows, eligible, true)?;
    Some(super::types::DomainIdentity {
        population,
        column: *columns.get(column.0 as usize)?,
    })
}

/// Rows never acquire identity from pointers or cardinality alone; native and semantic populations differ.
fn population(
    input: ryo_values::domain::plan_parts::types::Input,
    rows: &Rows<'_>,
    eligible: &[ryo_language::domain::relations::types::Relation],
    native: bool,
) -> Option<(super::types::Population, Vec<ColumnIndex>)> {
    use crate::domain::table_cells::types::TreeForm;
    use ryo_language::domain::relations::{service::kinds, types::Relation};
    use ryo_values::domain::plan_parts::types::Input;
    let (relation, kinds, count, native_file, selected, positions, columns) = match rows {
        Rows::Stored(view) => {
            let Input::Relation(relation) = input else {
                return None;
            };
            if !eligible.contains(&relation)
                || !crate::domain::table_model::service::rectangular(*view)
            {
                return None;
            }
            (
                relation,
                view.kinds,
                count_of(rows).0,
                None,
                None,
                None,
                (0..view.kinds.len())
                    .map(|column| ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX)))
                    .collect(),
            )
        }
        Rows::Selected(view) => {
            let relation = view.selection.relation;
            let count = view
                .source
                .columns
                .first()
                .map_or(0, |column| column.0.len());
            if !eligible.contains(&relation)
                || !crate::domain::table_model::service::rectangular(view.source)
                || view.selection.rows.iter().any(|row| *row as usize >= count)
            {
                return None;
            }
            (
                relation,
                view.source.kinds,
                count,
                None,
                Some(Arc::clone(&view.selection.identity)),
                Some(Arc::clone(&view.selection.rows)),
                view.selection.columns.clone(),
            )
        }
        Rows::Tree(tree) if native => {
            let relation = match tree.form {
                TreeForm::Elements => Relation::SyntaxTree,
                TreeForm::Edges => Relation::SyntaxChild,
            };
            if !eligible.contains(&relation) {
                return None;
            }
            let schema = kinds(relation);
            (
                relation,
                schema,
                count_of(rows).0,
                Some(tree.file),
                None,
                None,
                (0..schema.len())
                    .map(|column| ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX)))
                    .collect(),
            )
        }
        Rows::Tree(_) | Rows::Computed(_) => return None,
    };
    Some((
        super::types::Population {
            relation,
            kinds,
            rows: count,
            native_file,
            selected,
            positions,
        },
        columns,
    ))
}

/// Build both adjacency orientations from only endpoint/partition cells, retaining original row order.
#[cfg(test)]
pub(crate) fn build(
    identity: GraphIdentity,
    rows: &Rows<'_>,
    endpoints: [ColumnIndex; 2],
    keys: &[ColumnIndex],
) -> Result<Option<CompactGraph>, OperatingError> {
    build_borrowed(
        identity,
        crate::domain::operator::contract::views::BorrowedRows::from(rows),
        endpoints,
        keys,
    )
}
/// Build directly from every admitted bulk representation without copying its population.
pub(crate) fn build_borrowed(
    identity: GraphIdentity,
    rows: crate::domain::operator::contract::views::BorrowedRows<'_>,
    endpoints: [ColumnIndex; 2],
    keys: &[ColumnIndex],
) -> Result<Option<CompactGraph>, OperatingError> {
    check_columns(rows.kinds(), &endpoints)?;
    check_columns(rows.kinds(), keys)?;
    if rows.count() > (u32::MAX - 2) as usize {
        return Ok(None);
    }
    let mut graph = CompactGraph {
        identity,
        partitions: Vec::new(),
        partition_ids: Default::default(),
        vertices: Vec::new(),
        vertex_partitions: Vec::new(),
        vertex_ids: Default::default(),
        offsets: [Vec::new(), Vec::new()],
        neighbors: [Vec::new(), Vec::new()],
        forests: [None, None],
    };
    let mut pairs = Vec::new();
    let mut tuple = Vec::with_capacity(keys.len());
    for position in 0..rows.count() {
        let position = crate::domain::table_cells::types::RowIndex(position);
        tuple.clear();
        let mut complete = true;
        for column in keys {
            if let Some(value) = rows.cell(position, *column) {
                tuple.push(value);
            } else {
                complete = false;
                break;
            }
        }
        if !complete {
            continue;
        }
        let partition = if let Some(id) = graph.partition_ids.get(tuple.as_slice()) {
            *id
        } else {
            let Ok(id) = u32::try_from(graph.partitions.len()) else {
                return Ok(None);
            };
            let key: Arc<[Value]> = Arc::from(tuple.as_slice());
            graph.partitions.push(Arc::clone(&key));
            let _ = graph.partition_ids.insert(key, id);
            id
        };
        let mut vertices = [0_u32; 2];
        for (slot, column) in vertices.iter_mut().zip(endpoints) {
            let Some(value) = rows.cell(position, column) else {
                complete = false;
                break;
            };
            *slot = if let Some(vertex) = graph.vertex_ids.get(&(partition, value)) {
                *vertex
            } else {
                if graph.vertices.len() >= (u32::MAX - 2) as usize {
                    return Ok(None);
                }
                let id = u32::try_from(graph.vertices.len()).unwrap_or(u32::MAX);
                graph.vertices.push(value);
                graph.vertex_partitions.push(partition);
                let _ = graph.vertex_ids.insert((partition, value), id);
                id
            };
        }
        if complete {
            pairs.push(vertices);
        }
    }
    for orientation in 0..2 {
        let mut offsets = vec![0_u32; graph.vertices.len() + 1];
        for pair in &pairs {
            offsets[pair[orientation] as usize + 1] += 1;
        }
        for vertex in 1..offsets.len() {
            offsets[vertex] += offsets[vertex - 1];
        }
        let mut written = offsets.clone();
        let mut neighbors = vec![0_u32; pairs.len()];
        for pair in &pairs {
            let source = pair[orientation] as usize;
            neighbors[written[source] as usize] = pair[1 - orientation];
            written[source] += 1;
        }
        graph.forests[orientation] = forest(&offsets, &neighbors);
        graph.offsets[orientation] = offsets;
        graph.neighbors[orientation] = neighbors;
    }
    Ok(Some(graph))
}

/// Validate duplicate-insensitive single-parent ancestry and cycles iteratively once per graph.
fn forest(offsets: &[u32], neighbors: &[u32]) -> Option<Vec<u32>> {
    use super::types::traversal::value::ABSENT;
    let mut parents = vec![ABSENT; offsets.len().saturating_sub(1)];
    for vertex in 0..parents.len() {
        let outgoing = &neighbors[offsets[vertex] as usize..offsets[vertex + 1] as usize];
        if let Some(parent) = outgoing.first() {
            if outgoing.iter().any(|other| other != parent) {
                return None;
            }
            parents[vertex] = *parent;
        }
    }
    let mut state = vec![0_u8; parents.len()];
    let mut path = Vec::new();
    for start in 0..parents.len() {
        if state[start] != 0 {
            continue;
        }
        path.clear();
        let mut current = start;
        loop {
            if state[current] == 1 {
                return None;
            }
            if state[current] == 2 {
                break;
            }
            state[current] = 1;
            path.push(current);
            if parents[current] == ABSENT {
                break;
            }
            current = parents[current] as usize;
        }
        for vertex in &path {
            state[*vertex] = 2;
        }
    }
    Some(parents)
}
