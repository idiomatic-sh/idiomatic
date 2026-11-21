//! The constants of the `made_tables` concept.

use crate::domain::indexes::types::HashShift;

/// How far a row's hash is folded onto itself before the map of the rows held reads its low bits.
pub(crate) const HASH_FOLD: HashShift = HashShift(32);
