//! Tarjan cycle semantics: the nodes of a component of more than one, or with an edge to themselves.
use super::types::{
    CycleDriver, CycleMark, CycleScratch, EdgePlace, MeetOrder, NodeCount, NodeNumber, WalkStep,
};

/// Marks in `scratch.found`, by node number, each node of `nodes` in a component of more than one or with an edge to itself,
/// `edge` answering the node the edge at a place leaving a node leads to. Tarjan's strongly connected components, one pass.
pub(crate) fn components(
    nodes: NodeCount,
    edge: impl Fn(NodeNumber, EdgePlace) -> Option<NodeNumber>,
    scratch: &mut CycleScratch,
) {
    let count = nodes.0;
    scratch.order.clear();
    scratch.order.resize(count, UNMET);
    scratch.low.clear();
    scratch.low.resize(count, MeetOrder(0));
    scratch.open.clear();
    scratch.open.resize(count, CycleMark::Clear);
    scratch.found.clear();
    scratch.found.resize(count, CycleMark::Clear);
    scratch.stack.clear();
    scratch.walk.clear();
    mark_components(nodes, edge, scratch);
}

const UNMET: MeetOrder = MeetOrder(usize::MAX);

/// Tarjan's walk: every node once, marking in `found` those, and marks in `found` those of a component of more than one or with an edge to themselves.
#[expect(
    clippy::indexing_slicing,
    reason = "every index is a node number below the node count, to which `order`, `low`, `open` and `found` were sized before the walk"
)]
fn mark_components(
    nodes: NodeCount,
    lead: impl Fn(NodeNumber, EdgePlace) -> Option<NodeNumber>,
    room: &mut CycleScratch,
) {
    let CycleScratch {
        order,
        low,
        open,
        stack,
        found,
        walk,
        component,
        ..
    } = room;
    let nodes = nodes.0;
    let mut next = 0_usize;
    for root in 0..nodes {
        if order[root] != UNMET {
            continue;
        }
        walk.push(WalkStep {
            node: NodeNumber(root),
            edge: EdgePlace(0),
        });
        order[root] = MeetOrder(next);
        low[root] = MeetOrder(next);
        next += 1;
        stack.push(NodeNumber(root));
        open[root] = CycleMark::Raised;
        while let Some(&WalkStep {
            node: NodeNumber(node),
            edge,
        }) = walk.last()
        {
            if let Some(NodeNumber(to)) = lead(NodeNumber(node), edge) {
                if let Some(top) = walk.last_mut() {
                    top.edge.0 += 1;
                }
                if to == node {
                    found[node] = CycleMark::Raised;
                }
                // A node not yet walked is not open, so the two cases are apart.
                if open[to] == CycleMark::Raised {
                    low[node] = low[node].min(order[to]);
                }
                if order[to] == UNMET {
                    order[to] = MeetOrder(next);
                    low[to] = MeetOrder(next);
                    next += 1;
                    stack.push(NodeNumber(to));
                    open[to] = CycleMark::Raised;
                    walk.push(WalkStep {
                        node: NodeNumber(to),
                        edge: EdgePlace(0),
                    });
                }
                continue;
            }
            let _: Option<WalkStep> = walk.pop();
            if let Some(&WalkStep {
                node: NodeNumber(above),
                ..
            }) = walk.last()
            {
                low[above] = low[above].min(low[node]);
            }
            if low[node] == order[node] {
                component.clear();
                while let Some(member) = stack.pop() {
                    open[member.0] = CycleMark::Clear;
                    component.push(member);
                    if member.0 == node {
                        break;
                    }
                }
                if component.len() > 1 {
                    for member in component.iter() {
                        found[member.0] = CycleMark::Raised;
                    }
                }
            }
        }
    }
}

/// Marks the nodes on a cycle of `graph` and leaves their values ascending in `scratch.ordered`.
pub(crate) fn cycles(graph: &dyn CycleDriver, scratch: &mut CycleScratch) {
    let nodes = graph.nodes().0;
    components(graph.nodes(), |node, place| graph.edge(node, place), scratch);
    scratch.ordered.clear();
    scratch.ordered.extend(
        (0..nodes)
            .zip(&scratch.found)
            .filter(|(_, on)| **on == CycleMark::Raised)
            .filter_map(|(node, _)| graph.value(NodeNumber(node))),
    );
    scratch.ordered.sort_unstable();
}

/// Marks the nodes on a cycle of the engine's prepared adjacency and emits each, ascending, as both the
/// source and the destination of its row. The step is fed every edge row: the adjacency holds them all.
pub(crate) fn evaluate(
    driver: &mut dyn crate::domain::cycles::types::CyclesEvaluation,
    scratch: &mut CycleScratch,
) -> Result<(), crate::domain::contract::types::PortableOperatorFault> {
    use ryo_values::domain::plan_parts::types::InputNumber;
    cycles(driver.cycles()?, scratch);
    let edges = driver.population(InputNumber(0))?;
    driver.found(
        InputNumber(0),
        ryo_relations::domain::relations::types::Count(edges.0),
    );
    for value in &scratch.ordered {
        driver.emit(&[*value, *value])?;
    }
    Ok(())
}

/// The graph of a cycle search over edges whose `source` leads to `destination`: its one lowering, Tarjan's components
/// over its edges.
pub(crate) const fn lower(
    source: ryo_values::domain::plan_parts::types::ColumnIndex,
    destination: ryo_values::domain::plan_parts::types::ColumnIndex,
) -> super::types::CycleParts {
    super::types::CycleParts {
        source,
        destination,
    }
}
