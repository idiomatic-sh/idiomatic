//! The passes in their one order.

use crate::domain::lowered::types::LoweredPack;
use crate::domain::statistics::types::Statistics;

/// Runs every pass over the pack: prefixes, fusing, memos, numbering, ordering, hoisting. `statistics` is none for a load
/// given none: the lowering's order is then kept.
pub(crate) fn passed(pack: &mut LoweredPack, statistics: Option<&Statistics>) {
    super::prefixes::service::passed(pack);
    super::fusing::service::passed(pack);
    super::memos::service::passed(pack);
    super::numbering::service::passed(pack);
    super::ordering::service::passed(pack, statistics);
    super::hoisting::service::passed(pack);
}
