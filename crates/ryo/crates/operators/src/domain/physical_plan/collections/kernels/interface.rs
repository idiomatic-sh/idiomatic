//! The shape every item of the `kernels` collection implements.

use crate::domain::physical_plan::types::lowering::value::Lowering;

/// What every item of the `kernels` collection offers: whether a lowering runs it.
pub(crate) trait KernelsItem {
    /// Whether a node lowered to `lowering` runs this kernel.
    fn reached_by(&self, lowering: &Lowering) -> Reach;
}

/// Whether a node runs a kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reach {
    /// The node runs it.
    Runs,
    /// The node does not run it.
    Skips,
}
