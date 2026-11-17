//! The `growth` service: the fan a join's proven keys answer, the one algebra that composes a plan's declarations into bounds,
//! and the one check that refuses a node whose rows grow past its inputs'.
#[cfg(test)]
use super::errors::GrowthError;
use super::types::Fan;
#[cfg(test)]
use super::types::Growth;
#[cfg(test)]
use super::types::{Bound, Degree, NodeBound};
#[cfg(test)]
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
#[cfg(test)]
use crate::domain::physical_plan::types::{EngineUses, NodeReaders, PlanNodeId, RowSource};
use ryo_values::domain::plan_parts::types::KeyFan;

/// The fan a join's proven keys answer for each left row: the one mapping a probe and a band join read.
#[must_use]
pub(crate) const fn fan_of(key: KeyFan) -> Fan {
    match key {
        KeyFan::Unique => Fan::PerLeft,
        KeyFan::Shared => Fan::Repeats,
        KeyFan::FileOnly => Fan::FileProduct,
        KeyFan::AllPairs => Fan::DeclaredPairs,
    }
}

/// The bound of the rows a node reads from `source`: the rows of the partition for facts, tree, frontier and observation rows,
/// and the bound of the node it names. The one place a source's bound is decided.
#[cfg(test)]
pub(crate) fn source_bound(source: RowSource, made: &[NodeBound]) -> Bound {
    match source {
        RowSource::Node(node) => made
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
            .map_or(ROWS, |bound| bound.output),
        RowSource::Facts(_)
        | RowSource::Tree(_)
        | RowSource::Frontier(_)
        | RowSource::Observation(_) => ROWS,
    }
}

/// `rows^1`: the bound of a relation's rows.
#[cfg(test)]
const ROWS: Bound = Bound {
    rows: Degree(1),
    depth: Degree(0),
    repeats: Degree(0),
};

/// The larger of two bounds, degree by degree.
#[cfg(test)]
fn larger(a: Bound, b: Bound) -> Bound {
    Bound {
        rows: a.rows.max(b.rows),
        depth: a.depth.max(b.depth),
        repeats: a.repeats.max(b.repeats),
    }
}

/// The product of two bounds: their degrees added.
#[cfg(test)]
fn times(a: Bound, b: Bound) -> Bound {
    Bound {
        rows: Degree(a.rows.0.saturating_add(b.rows.0)),
        depth: Degree(a.depth.0.saturating_add(b.depth.0)),
        repeats: Degree(a.repeats.0.saturating_add(b.repeats.0)),
    }
}

/// The bound of a node of `growth` over inputs bounded by `inputs`, in the order it reads them; a node that reads no node (a
/// scan, a lookup) reads the partition's rows. The one algebra: work is the inputs and the output together, the log of a sort not
/// counted.
#[cfg(test)]
pub(crate) fn composed(growth: Growth, inputs: &[Bound]) -> NodeBound {
    let first = inputs.first().copied().unwrap_or(ROWS);
    let second = inputs.get(1).copied().unwrap_or(first);
    let read = inputs.iter().copied().fold(first, larger);
    let output = match growth {
        Growth::AtMostInput => read,
        Growth::PerRowText => ROWS,
        Growth::OnePerLeftRow | Growth::GroupOutput => first,
        Growth::Keyed(fan) | Growth::Banded(fan) => match fan {
            Fan::PerLeft => first,
            Fan::Repeats => times(
                first,
                Bound {
                    repeats: Degree(1),
                    ..Bound::default()
                },
            ),
            Fan::FileProduct | Fan::DeclaredPairs => times(first, second),
        },
        Growth::PerStartWalk => times(
            first,
            Bound {
                depth: Degree(1),
                ..Bound::default()
            },
        ),
    };
    NodeBound {
        output,
        work: larger(read, output),
    }
}

/// Every live node of `nodes` (one that is read or kept by the engine) whose output or work grows faster in rows than the
/// largest of its inputs, past `rows^1`, refused; a node the query declared every pair of is not. Nodes come in id order, each
/// reading only earlier ones, so each input's bound is known when its reader is judged.
///
/// # Errors
///
/// `Superlinear` naming the first such node.
#[cfg(test)]
pub(crate) fn checked(nodes: &[PhysicalNode]) -> Result<Vec<NodeBound>, GrowthError> {
    let mut made = Vec::with_capacity(nodes.len());
    for (node, id) in nodes.iter().zip((0..).map(PlanNodeId)) {
        let growth = node.lowering.growth();
        let inputs: Vec<Bound> = node
            .lowering
            .sources()
            .map(|source| source_bound(source, &made))
            .collect();
        let bound = composed(growth, &inputs);
        let widest = inputs
            .iter()
            .map(|input| input.rows)
            .max()
            .unwrap_or(Degree(1));
        let degree = bound.output.rows.max(bound.work.rows);
        let live = node.readers != NodeReaders(0) || node.engine != EngineUses(0);
        let declared = matches!(
            growth,
            Growth::Keyed(Fan::DeclaredPairs) | Growth::Banded(Fan::DeclaredPairs)
        );
        if live && !declared && degree > Degree(1) && degree > widest {
            return Err(GrowthError::Superlinear {
                node: id,
                growth,
                degree,
            });
        }
        made.push(bound);
    }
    Ok(made)
}
