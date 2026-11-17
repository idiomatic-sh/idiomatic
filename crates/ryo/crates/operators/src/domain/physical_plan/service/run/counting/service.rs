//! The runner's one record of each making: every making is held to the run budget and the run's worst making kept, counting
//! on or off; each span it times and each making it counts goes into the worker's buffers when counting is on, and costs one
//! check of the switch when it is off.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::growth::errors::GrowthError;
use crate::domain::growth::service::{against_worst, fed_rows, within_budget};
use crate::domain::growth::types::Making;
use crate::domain::physical_plan::types::PlanNodeId;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::node_counters::value::{MadeIn, OpenSpan, WorstMaking};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_values::domain::rows::types::RowCount;

/// Opens a timed span when counting is on.
pub(crate) fn timed(buffers: &mut FileBuffers) -> Option<OpenSpan> {
    buffers.counters.as_mut().map(|counters| counters.opened())
}

/// Closes `span`, opened by `timed`, and counts its own time to `node`.
pub(crate) fn timed_to(buffers: &mut FileBuffers, span: Option<OpenSpan>, node: PlanNodeId) {
    if let (Some(counters), Some(span)) = (buffers.counters.as_mut(), span) {
        counters.closed(span, node);
    }
}

/// `node` of `plan` was made once in the open file, `made_in` an open span or as a chain's stage, and wrote `out` rows: the
/// rows it wrote in the open unit are held to the run budget against what it was fed, the making kept as the run's worst
/// when it is, and counted when counting is on.
///
/// # Errors
///
/// `OverBudget` when its rows went past the run budget, or past its operator's declaration to write at most its input; the
/// buffers' worst making then names the node.
pub(crate) fn counted(
    plan: &PhysicalPlan,
    buffers: &mut FileBuffers,
    node: PlanNodeId,
    made_in: MadeIn,
    out: RowCount,
) -> Result<(), PortableOperatorFault> {
    let fed = buffers.fed(node);
    let place = usize::try_from(node.0).unwrap_or(usize::MAX);
    let wrote = buffers.wrote.get_mut(place).map_or(out, |wrote| {
        wrote.0 = wrote.0.saturating_add(out.0);
        *wrote
    });
    buffers.touched.touched(node);
    let making = Making {
        fed: fed_rows(&fed.0),
        wrote,
    };
    let worst = buffers.worst.map(|worst| Making {
        fed: worst.fed,
        wrote: worst.wrote,
    });
    if against_worst(making, worst) == std::cmp::Ordering::Greater {
        buffers.worst = Some(WorstMaking {
            node,
            fed: making.fed,
            wrote,
        });
    }
    if let Some(counters) = buffers.counters.as_mut() {
        counters.made(node, made_in, fed, out);
    }
    let physical = plan
        .nodes
        .get(place)
        .ok_or(PortableOperatorFault::WrongOperation)?;
    within_budget(physical.lowering.growth(), making.fed, wrote).map_err(|error| match error {
        GrowthError::OverBudget { fed, wrote } => PortableOperatorFault::OverBudget { fed, wrote },
        // The budget answers no plan refusal: that is the plan check's, which no making reaches.
        #[cfg(test)]
        GrowthError::Superlinear { .. } => PortableOperatorFault::WrongOperation,
    })
}
