//! The passes in their one order.

use crate::domain::lowered::types::LoweredPack;
use crate::domain::statistics::types::Statistics;

/// Runs every pass over the pack: prefixes, fusing, memos, numbering, ordering, hoisting.
pub(crate) fn passed(pack: &mut LoweredPack, statistics: &Statistics) {
    super::prefixes::service::passed(pack);
    super::fusing::service::passed(pack);
    super::memos::service::passed(pack);
    super::numbering::service::passed(pack);
    super::ordering::service::passed(pack, statistics);
    super::hoisting::service::passed(pack);
}
