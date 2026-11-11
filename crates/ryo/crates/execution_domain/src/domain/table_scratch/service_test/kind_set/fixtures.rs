//! The support of the `kind_set` cases.
use crate::domain::table_scratch::types::{KindMask, KindSet};
use ryo_relations::domain::relations::types::KindBits;

/// The set every row asks: kind 2 alone, held as bit 2 of the first mask word.
pub(super) fn set() -> KindSet {
    KindSet {
        bits: KindMask(vec![0b100]),
        kinds: vec![KindBits(2)],
    }
}
