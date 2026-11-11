//! The types of the `credit` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::super::types::HoldCount;
use crate::application::file_dag::derived::ledger::service::ResourceLease;
use std::sync::Arc;

/// A handle being released, with how many of its clones are being released with it.
pub(super) struct Dropping<'a> {
    pub(crate) lease: &'a Arc<ResourceLease>,
    pub(crate) handles: HoldCount,
}
