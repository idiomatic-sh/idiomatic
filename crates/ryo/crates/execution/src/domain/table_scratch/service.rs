//! Membership in a prepared kind set; invocation buffers are dropped by their owner.

use super::types::KindSet;
use crate::domain::table_model::types::SetMembership;
use ryo_relations::domain::relations::types::KindBits;

/// Whether the kind numbered `kind` is in the set.
#[must_use]
pub(crate) fn kind_has(set: &KindSet, kind: KindBits) -> SetMembership {
    let kind = kind.0;
    let in_set = set
        .bits
        .0
        .get(usize::from(kind) / 64)
        .is_some_and(|word| (word >> (kind % 64)) & 1 == 1);
    if in_set {
        SetMembership::In
    } else {
        SetMembership::Out
    }
}
