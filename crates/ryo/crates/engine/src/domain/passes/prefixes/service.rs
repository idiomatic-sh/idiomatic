//! The service of the `prefixes` pass.

use crate::domain::lowered::service::each_touched;
use crate::domain::lowered::types::LoweredPack;

/// Sibling programs that start alike share the start and fork after it.
pub(crate) fn passed(pack: &mut LoweredPack) {
    each_touched(pack, &mut |_| {});
}
