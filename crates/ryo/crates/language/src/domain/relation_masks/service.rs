//! The `relation_masks` service: free functions named by verb over a set of relations, with no IO and no ports.

use super::constants::WORDS;
use super::types::RelationSet;
use ryo_relations::domain::relations::types::Count;

/// The set of no relation.
#[must_use]
pub const fn none() -> RelationSet {
    RelationSet([0; WORDS])
}

/// Sets the bit of the relation numbered `place`.
pub fn add(mask: &mut RelationSet, place: Count) {
    if let Some(word) = mask.0.get_mut(place.0 / 64) {
        *word |= 1 << (place.0 % 64);
    }
}

/// The numbers of the relations in the set, in ascending order.
pub fn places(mask: &RelationSet) -> impl Iterator<Item = Count> + '_ {
    mask.0.iter().enumerate().flat_map(|(word, bits)| {
        let mut bits = *bits;
        std::iter::from_fn(move || {
            if bits == 0 {
                return None;
            }
            let place = word * 64 + usize::try_from(bits.trailing_zeros()).unwrap_or(usize::MAX);
            bits &= bits - 1;
            Some(Count(place))
        })
    })
}
