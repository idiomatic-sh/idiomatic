//! The service of the `memos` pass.

use crate::domain::lowered::service::each_touched;
use crate::domain::lowered::types::LoweredPack;

/// A step over an inline body that reads no register but its keys is kept for the file by them.
pub(crate) fn passed(pack: &mut LoweredPack) {
    each_touched(pack, &mut |_| {});
}
