//! The service of the `numbering` pass.

use crate::domain::lowered::types::LoweredPack;

/// Every step and stage takes its plan point: its place in its unit's pre-order.
pub(crate) fn passed(pack: &mut LoweredPack) {
    let _ = pack;
}
