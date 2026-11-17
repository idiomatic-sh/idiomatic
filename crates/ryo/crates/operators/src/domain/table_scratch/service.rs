//! Membership in a prepared kind set; invocation buffers are dropped by their owner.

use super::types::KindSet;
use crate::domain::table_scratch::types::{Memo, MemoSlot, MemoTable, SetMembership};
use ryo_relations::domain::relations::types::KindBits;

/// Whether the kind numbered `kind` is in the set.
#[must_use]
pub fn kind_has(set: &KindSet, kind: KindBits) -> SetMembership {
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

/// What `memo` holds at `place`; none past its end, where nothing was decided.
pub(crate) fn decision(memo: &MemoTable, place: MemoSlot) -> Option<Memo> {
    memo.decided.get(place.0).copied()
}

/// Remembers `decided` at `place` of `memo`, growing it to the place.
pub(crate) fn remember(memo: &mut MemoTable, place: MemoSlot, decided: Memo) {
    if memo.decided.len() <= place.0 {
        memo.decided
            .resize(place.0.saturating_add(1), Memo::Unknown);
    }
    if let Some(held) = memo.decided.get_mut(place.0) {
        if *held == Memo::Unknown {
            memo.written.push(place);
        }
        *held = decided;
    }
}

/// Forgets every decision of `memo`: only the places decided since the run began are written.
pub(crate) fn forgotten(memo: &mut MemoTable) {
    for place in memo.written.drain(..) {
        if let Some(held) = memo.decided.get_mut(place.0) {
            *held = Memo::Unknown;
        }
    }
}
