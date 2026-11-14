//! The runner's nodes that mint or render through the resources while they read their rows: a `text_map` stage and a
//! `render`, which read a tree relation's rows from a copy, never from the elements in place.
use super::super::reading::service::{pending_of, returned, rows_in};
use super::super::types::Reading;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::render::item::rendered;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::facts;
use crate::domain::physical_plan::collections::kernels::scan_node::item::{hold, take};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::{copied, roots};
use crate::domain::physical_plan::collections::kernels::text_map::item::{RowMapRun, mapped};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{
    KernelRun, Lowering, NodeReaders, PlanNodeId, RowSource,
};
use ryo_values::domain::rows::types::Table;

/// Runs a node that mints or renders through the resources while it reads its rows (a `text_map` stage or a `render`) and
/// holds its rows and pending fault. It holds the resources mutably, so it reads a tree relation's rows from a copy, never
/// from the elements in place.
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
    };
    let pending = pending_of(&input);
    let mut output = Table::default();
    let made = match lowering {
        // A render reads every row before it answers one: its input's fault comes before any of its rows.
        Lowering::Render { render, .. } => match &pending {
            Some(fault) => Ok(Some(fault.clone())),
            None => Ok(rendered(render, rows_in(&input), run.resources, &mut output).err()),
        },
        Lowering::TextMap { map, .. } => match buffers
            .decisions
            .get_mut(usize::try_from(node.0).unwrap_or(usize::MAX))
        {
            Some(decisions) => mapped(
                map,
                rows_in(&input),
                &mut RowMapRun {
                    resources: &mut *run.resources,
                    matcher: run.matcher,
                    decisions,
                    scratch: &mut buffers.scratch,
                    places: &mut buffers.places,
                    output: &mut output,
                },
            ),
            None => Err(PortableOperatorFault::WrongOperation),
        },
        Lowering::Filter { .. }
        | Lowering::Lookup { .. }
        | Lowering::Project { .. }
        | Lowering::Extend { .. }
        | Lowering::Probe { .. }
        | Lowering::Distinct { .. }
        | Lowering::Group { .. }
        | Lowering::OrderedMatch { .. }
        | Lowering::Unlowered => Err(PortableOperatorFault::WrongOperation),
    };
    returned(plan, buffers, input, NodeReaders(1));
    let fault = made?;
    hold(plan, buffers, node, output, fault.or(pending));
    Ok(())
}
