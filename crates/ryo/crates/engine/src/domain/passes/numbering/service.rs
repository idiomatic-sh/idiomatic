//! The service of the `numbering` pass.

use crate::domain::lowered::service::each_touched;
use crate::domain::lowered::types::LoweredPack;

/// Every step and stage takes its plan point: its place in its unit's pre-order.
pub(crate) fn passed(pack: &mut LoweredPack) {
    each_touched(pack, &mut |_| {});
}
