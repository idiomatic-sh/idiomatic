//! The `growth` service: the fan a join's proven keys answer, the one algebra that composes a plan's declarations into bounds,
//! the one check that refuses a node whose rows grow past its inputs', and the run budget's one check of a unit's makings of a
//! node, with the one order of makings by how far they fan out.
use super::constants::RUN_BUDGET;
use super::errors::GrowthError;
use super::types::{Fan, Growth, Making};
#[cfg(test)]
use super::types::{Bound, Degree, NodeBound};
#[cfg(test)]
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
#[cfg(test)]
use crate::domain::physical_plan::types::{EngineUses, NodeReaders, PlanNodeId, RowSource};
use ryo_values::domain::plan_parts::types::KeyFan;
use ryo_values::domain::rows::types::RowCount;
use std::cmp::Ordering;

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

/// Whether a unit's makings of one node, of an operator whose instance grows as `growth`, fed `fed` rows and writing `wrote`
/// rows, are within the run budget. The unit is whatever its caller makes at once: a file today.
///
/// # Errors
///
/// `OverBudget` with the rows fed and written: when, of an operator not bounded by the text it reads, they wrote more than
/// the budget allows for the rows they were fed and more than the budget's floor, or when their operator is declared to write
/// at most its input and they wrote more than they were fed.
pub(crate) fn within_budget(growth: Growth, fed: RowCount, wrote: RowCount) -> Result<(), GrowthError> {
    let allowed = fed.0.max(1).saturating_mul(RUN_BUDGET.per_fed_row.0);
    // A text map writes rows by the text it reads, not by its rows: its bound is the file's bytes, which the growth tests
    // hold it to, so the rows per row fed do not bound it.
    let past_budget = growth != Growth::PerRowText
        && wrote.0 > allowed
        && wrote.0 > RUN_BUDGET.floor.0;
    let past_declaration = growth == Growth::AtMostInput && wrote.0 > fed.0;
    if past_budget || past_declaration {
        return Err(GrowthError::OverBudget { fed, wrote });
    }
    Ok(())
}

/// Where `making` falls against the run's worst making so far: `Greater` when it becomes the worst, the one that wrote the
/// most rows per row fed of those that wrote at least the budget's floor.
pub(crate) fn against_worst(making: Making, worst: Option<Making>) -> Ordering {
    if making.wrote.0 < RUN_BUDGET.floor.0 {
        return Ordering::Less;
    }
    worst.map_or(Ordering::Greater, |worst| fan_out(making, worst))
}

/// The rows a making was fed, every input summed.
pub(crate) fn fed_rows(fed: &[RowCount]) -> RowCount {
    RowCount(
        fed.iter()
            .fold(0_usize, |sum, rows| sum.saturating_add(rows.0)),
    )
}

/// How `making`'s rows written per row fed compare with `than`'s, a making fed no row counted as fed one.
fn fan_out(making: Making, than: Making) -> Ordering {
    let rows = |count: RowCount| u128::try_from(count.0).unwrap_or(u128::MAX);
    rows(making.wrote)
        .saturating_mul(rows(than.fed).max(1))
        .cmp(&rows(than.wrote).saturating_mul(rows(making.fed).max(1)))
}
