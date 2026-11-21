//! The service of the `hoisting` pass.

use crate::domain::lowered::service::each_touched;
use crate::domain::lowered::types::LoweredPack;

/// A segment that reads no register of the loop around it runs once per file, or once per key.
pub(crate) fn passed(pack: &mut LoweredPack) {
    each_touched(pack, &mut |_| {});
}
