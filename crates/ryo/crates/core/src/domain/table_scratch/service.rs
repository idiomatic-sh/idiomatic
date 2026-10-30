//! The `table_scratch` service: giving back the room a big file made the operators grow, and asking of a set of kinds, with no IO and no ports.

use ryo_language::domain::relations::types::KindBits;

use super::types::KindSet;
use crate::domain::table_model::types::{KeyIndex, SetMembership, Table};
use crate::domain::table_scratch::types::OperatorScratch;

/// How many cells a room of a worker keeps at most between files, which is 512 KB of cells: what is under it is the worker's to the
/// end of the run, and only what a file past it grew a room to is given back, so the calls that allocate a big block are
/// those of the few files that need one.
pub(crate) const ROOM: usize = 1 << 18;

/// Gives back what the table grew past [`ROOM`] cells.
pub(crate) fn trim_table(table: &mut Table) {
    if table.cells.capacity() > ROOM {
        table.cells.shrink_to(ROOM);
    }
}

/// Gives back what the order grew past [`ROOM`] rows.
pub(crate) fn trim_order(order: &mut KeyIndex) {
    order.rows.shrink_to(ROOM);
    order.hashes.shrink_to(ROOM);
    order.heads.shrink_to(ROOM);
    order.next.shrink_to(ROOM);
    order.sorted.shrink_to(ROOM);
    order.pairs.shrink_to(ROOM);
    order.runs.shrink_to(ROOM);
}

/// Gives back what a huge file made the scratch grow past [`ROOM`].
pub(crate) fn trim_scratch(scratch: &mut OperatorScratch) {
    trim_table(&mut scratch.right);
    trim_table(&mut scratch.domain);
    trim_order(&mut scratch.order);
    trim_order(&mut scratch.domain_order);
    scratch.row.shrink_to(ROOM);
    scratch.other.shrink_to(ROOM);
    scratch.marks.0.shrink_to(ROOM);
    scratch.candidates.0.shrink_to(ROOM);
    scratch.nearest.0.shrink_to(ROOM);
    scratch.climbing.0.shrink_to(ROOM);
    scratch.hashes.0.shrink_to(ROOM);
    scratch.captured.shrink_to(ROOM);
    scratch.walk.frontier.shrink_to(ROOM);
    scratch.walk.next.shrink_to(ROOM);
    scratch.walk.reached.shrink_to(ROOM);
    scratch.inner.frontier.shrink_to(ROOM);
    scratch.inner.next.shrink_to(ROOM);
    scratch.inner.reached.shrink_to(ROOM);
}

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
