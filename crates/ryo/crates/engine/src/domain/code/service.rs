//! The services of the `code` concept: a word from its slots, and a list of the pool. Each code's own constructor and reader are
//! in the part of its kind (`anchoring`, `walking`, `deciding`, `controlling`, `filling`), the pool's records in `records` and
//! the small codes in `small_codes`.

use super::types::{CodeWord, Op, Short, WordFlag, WordSlot};

/// A word of an operation with its operands in the layout of the operand table.
#[must_use]
pub(crate) const fn word(
    op: Op,
    flag: WordFlag,
    short: Short,
    a: WordSlot,
    b: WordSlot,
    x: WordSlot,
) -> CodeWord {
    CodeWord {
        op,
        flag,
        short,
        a,
        b,
        x,
    }
}

/// The list a pool offset names: its length is its first slot. A place the pool does not reach, or a length it does not hold,
/// is the empty list.
#[must_use]
pub(crate) fn list(pool: &[WordSlot], at: WordSlot) -> &[WordSlot] {
    let start = usize::try_from(at.0).unwrap_or(usize::MAX);
    let length = pool
        .get(start)
        .map_or(0, |count| usize::try_from(count.0).unwrap_or(0));
    pool.get(start.saturating_add(1)..start.saturating_add(1).saturating_add(length))
        .unwrap_or(&[])
}
