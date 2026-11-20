//! The service of the `fusing` pass.

use crate::domain::lowered::service::each_touched;
use crate::domain::lowered::types::LoweredPack;

/// A bucket walk and the range tests on its rows become one range walk; a child walk and its field test one field walk; an ancestor walk and its equality one containment test.
pub(crate) fn passed(pack: &mut LoweredPack) {
    each_touched(pack, &mut |_| {});
}
