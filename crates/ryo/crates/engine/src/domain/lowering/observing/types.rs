//! The types of the `observing` part.

use crate::domain::lowering::types::BoundProducer;
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::query_plans::types::NodeDigest;

/// Whose rows a step holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Local {
    /// The rows of the file being scanned: the union over the files is the step's rows over the run.
    File,
    /// The same rows for every file: a constant table.
    Constant,
}

/// A bound observation a rule reads, with the producers the load accepted and could shape the observed step of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedBinding {
    /// The binding, by its place among the plan's bound inputs.
    pub(crate) binding: ObservationBindingId,
    /// The digest the host names the observation's definition by.
    pub(crate) node: NodeDigest,
    /// The producers, in the host's order.
    pub(crate) producers: Vec<BoundProducer>,
}
