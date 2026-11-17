//! The plan's growth check: every live node's bound composed through the one algebra, in id order, and the first node whose rows
//! grow past its inputs' refused.
use crate::domain::growth::errors::GrowthError;
use crate::domain::growth::service::{ROWS, composed, judged};
use crate::domain::growth::types::{Bound, GrowthKind, GrowthNode, NodeBound};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{EngineUses, NodeReaders, PlanNodeId, RowSource};

/// The bound of the rows a node reads from `source`: the rows of the partition for facts, tree, frontier and observation rows,
/// and the bound of the node it names. The one place a source's bound is decided.
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

/// Every live node of `nodes` (one that is read or kept by the engine) whose output or work grows faster in rows than the
/// largest of its inputs, past `rows^1`, refused; a node the query declared every pair of is not. Nodes come in id order, each
/// reading only earlier ones, so each input's bound is known when its reader is judged.
///
/// # Errors
///
/// `Superlinear` naming the first such node.
pub(crate) fn checked(nodes: &[PhysicalNode]) -> Result<Vec<NodeBound>, GrowthError> {
    let mut made = Vec::with_capacity(nodes.len());
    for (node, id) in nodes.iter().zip((0..).map(PlanNodeId)) {
        let growth = node.lowering.growth();
        let inputs: Vec<Bound> = node
            .lowering
            .sources()
            .map(|source| source_bound(source, &made))
            .collect();
        let live = node.readers != NodeReaders(0) || node.engine != EngineUses(0);
        // A node a rewrite left unread is not judged: its bound is still made, for the nodes that read it.
        let bound = if live {
            judged(
                growth,
                &inputs,
                GrowthNode(id.0),
                GrowthKind(node.lowering.kind_name()),
            )?
        } else {
            composed(growth, &inputs)
        };
        made.push(bound);
    }
    Ok(made)
}
