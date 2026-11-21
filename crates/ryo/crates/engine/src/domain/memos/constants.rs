//! The constants of the `memos` concept.

use crate::domain::indexes::types::{HashShift, RowHash};

/// What a site's number is multiplied by before it is mixed into its key's hash, so that two sites asked the same key do
/// not meet in the map.
pub(crate) const SITE_MULTIPLIER: RowHash = RowHash(0xff51_afd7_ed55_8ccd);

/// How far an answer's hash is folded onto itself before the map reads its low bits.
pub(crate) const HASH_FOLD: HashShift = HashShift(29);
