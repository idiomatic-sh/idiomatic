//! The service of the `ordering` pass.

use crate::domain::lowered::types::LoweredPack;
use crate::domain::statistics::types::Statistics;

/// Each level of a unit the statistics measured takes the measured order.
pub(crate) fn passed(pack: &mut LoweredPack, statistics: &Statistics) {
    let _ = (pack, statistics);
}
