//! The runner's node counters: each span it times and each making it counts goes into the worker's buffers when counting is
//! on, and costs one check of the switch when it is off.
use crate::domain::physical_plan::types::PlanNodeId;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::node_counters::value::{MadeIn, OpenSpan};
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

/// `node` was made once in the open file, `made_in` an open span or as a chain's stage, and wrote `out` rows: counted, with
/// what it was fed, when counting is on.
pub(crate) fn counted(buffers: &mut FileBuffers, node: PlanNodeId, made_in: MadeIn, out: RowCount) {
    if buffers.counters.is_none() {
        return;
    }
    let fed = buffers.fed(node);
    if let Some(counters) = buffers.counters.as_mut() {
        counters.made(node, made_in, fed, out);
    }
}
