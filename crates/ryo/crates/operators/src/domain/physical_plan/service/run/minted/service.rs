//! The runner's node that renders through the resources while it reads its rows: a `render`, which reads a tree relation's
//! rows from a copy, never from the elements in place.
use super::super::reading::service::{pending_of, returned, rows_in};
use super::super::types::Reading;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::render::item::rendered;
use crate::domain::physical_plan::collections::kernels::scan_bound::item::held;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{count_of, facts};
use crate::domain::physical_plan::collections::kernels::scan_node::item::{fed_with, hold, take};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::{copied, roots};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::{Lowering, RenderNode};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{KernelRun, NodeFed, NodeReaders, PlanNodeId, RowSource};
use ryo_values::domain::rows::types::RowCount;

/// Runs a node that renders through the resources while it reads its rows (a `render`) and holds its rows and pending fault.
/// It holds the resources mutably, so it reads a tree relation's rows from a copy, never from the elements in place.
pub(crate) fn minted(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    node: PlanNodeId,
    lowering: &Lowering,
    source: RowSource,
    buffers: &mut FileBuffers,
) -> Result<(), PortableOperatorFault> {
    let inputs = run.inputs;
    let input = match source {
        RowSource::Tree(relation) => {
            Reading::Copied(copied(&roots(inputs, run.resources.tree(), node, relation)))
        }
        RowSource::Facts(relation) => Reading::Facts(facts(inputs, relation)?),
        RowSource::Node(read) => Reading::Node(read, take(buffers, read)?),
        // A withheld input faults where it is first read, never as empty rows.
        RowSource::Frontier(place) => Reading::Held(held(inputs.frontier(place))?),
        RowSource::Observation(place) => Reading::Held(held(inputs.observation(place))?),
    };
    fed_with(
        buffers,
        node,
        NodeFed([count_of(rows_in(&input)), RowCount(0), RowCount(0)]),
    );
    let pending = pending_of(&input);
    // The node's table: one the worker's pool kept, its room reused.
    let mut output = buffers.pool.fresh();
    let made = match lowering {
        // A render reads every row before it answers one: its input's fault comes before any of its rows.
        Lowering::Render(RenderNode { render, .. }) => match &pending {
            Some(fault) => Ok(Some(fault.clone())),
            None => Ok(rendered(render, rows_in(&input), run.resources, &mut output).err()),
        },
        // A text_map stage is run by the chain of streamed stages; the other kinds by the runner.
        Lowering::TextMap(_)
        | Lowering::Filter(_)
        | Lowering::Lookup(_)
        | Lowering::Project(_)
        | Lowering::Extend(_)
        | Lowering::Probe(_)
        | Lowering::Distinct(_)
        | Lowering::Group(_)
        | Lowering::OrderedMatch(_)
        | Lowering::BandJoin(_)
        | Lowering::Reach(_)
        | Lowering::SeededReach(_)
        | Lowering::Axis(_)
        | Lowering::OrderedWalk(_)
        | Lowering::Worklist(_)
        | Lowering::Components(_)
        | Lowering::ScanBound(_) => Err(PortableOperatorFault::WrongOperation),
    };
    returned(buffers, input, NodeReaders(1));
    let fault = made?;
    hold(plan, buffers, node, output, fault.or(pending));
    Ok(())
}
