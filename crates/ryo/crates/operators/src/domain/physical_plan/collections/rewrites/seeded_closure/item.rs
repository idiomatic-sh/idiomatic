//! Spike (ryo-fmx3): the `seeded_closure` item of the `rewrites` collection. A probe whose right rows are a closure node and
//! whose key binds the closure's partition and source asks the closure only for the sources its left rows bind: it becomes
//! a seeded reach over the closure's edges, which walks each bound source once and probes the rows it reached. The rows a
//! left row meets are the closure's rows of its partition and source, each once, ascending, as the closure holds them; a
//! closure no reader is left on is never made.

use super::super::interface::RewritesItem;
use crate::domain::equality_join::types::{ProbeKey, ProbeMode, SeededJoin};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{KeyedIndex, NodeReaders, ProbeIndex, RowSource};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// `seeded_closure`, one item of the `rewrites` collection.
pub(crate) struct SeededClosureItem;

impl RewritesItem for SeededClosureItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], _indexes: &mut Vec<KeyedIndex>) {
        if !crate::spike::lazy() || std::env::var_os("RYO_NO_SEEDED_CLOSURE").is_some() {
            return;
        }
        for at in 0..nodes.len() {
            let Some((closure, edges, seeded)) = seeded(nodes, at) else {
                continue;
            };
            if let Some(node) = nodes.get_mut(closure) {
                node.readers = NodeReaders(node.readers.0.saturating_sub(1));
            }
            if let RowSource::Node(read) = edges
                && let Some(node) = nodes.get_mut(usize::try_from(read.0).unwrap_or(usize::MAX))
            {
                node.readers = NodeReaders(node.readers.0.saturating_add(1));
            }
            if let Some(node) = nodes.get_mut(at) {
                node.lowering = seeded;
            }
        }
    }
}

/// The seeded reach the probe at `at` answers as, with the closure node it read and the edges it reads instead; none for any
/// probe not of the shape.
fn seeded(nodes: &[PhysicalNode], at: usize) -> Option<(usize, RowSource, Lowering)> {
    let Lowering::Probe {
        left,
        right: RowSource::Node(read),
        probe,
        index: ProbeIndex::Own,
    } = &nodes.get(at)?.lowering
    else {
        return None;
    };
    if !matches!(probe.mode, ProbeMode::Inner | ProbeMode::Semi | ProbeMode::Anti)
        || *left == RowSource::Node(*read)
    {
        return None;
    }
    let closure = usize::try_from(read.0).ok()?;
    let Lowering::Reach { edges, graph } = &nodes.get(closure)?.lowering else {
        return None;
    };
    let ProbeKey::Columns { right, .. } = &probe.key else {
        return None;
    };
    // The key binds every partition column and the source: columns 0..=keys of the closure's rows.
    let width = u32::try_from(graph.keys.len() + 2).ok()?;
    if !(0..width - 1).all(|column| right.contains(&ColumnIndex(column))) {
        return None;
    }
    Some((
        closure,
        *edges,
        Lowering::SeededReach {
            left: *left,
            edges: *edges,
            graph: graph.clone(),
            join: SeededJoin {
                probe: probe.clone(),
                columns: (0..width).map(ColumnIndex).collect(),
            },
            edge_index: None,
        },
    ))
}
