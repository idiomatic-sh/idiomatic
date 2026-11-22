//! THROWAWAY (ryo-fmx3.1 redo-alloc): a row's key of a few columns held inline (no allocation up to four cells), for the
//! grouping maps of the executor's operators. It hashes and compares exactly as the key's cells as a slice do (as a
//! `Vec<u64>` does), so a map keyed by it holds its entries in the same places and iterates them in the same order as the
//! `Vec<u64>`-keyed map it replaces: the rows come out as before.

use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

/// Cells held inline up to this many.
const INLINE: usize = 4;

#[derive(Clone)]
pub(super) enum Key {
    Inline(u8, [u64; INLINE]),
    Heap(Vec<u64>),
}

impl Key {
    pub(super) fn of(cells: &[u64]) -> Key {
        if cells.len() <= INLINE {
            let mut v = [0u64; INLINE];
            v[..cells.len()].copy_from_slice(cells);
            Key::Inline(cells.len() as u8, v)
        } else {
            Key::Heap(cells.to_vec())
        }
    }
    pub(super) fn cells(&self) -> &[u64] {
        match self {
            Key::Inline(n, v) => &v[..usize::from(*n)],
            Key::Heap(v) => v,
        }
    }
}

impl Borrow<[u64]> for Key {
    fn borrow(&self) -> &[u64] {
        self.cells()
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cells().hash(state);
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.cells() == other.cells()
    }
}

impl Eq for Key {}

/// A group's items, the first held inline (most groups of a row's key hold one row).
pub(super) enum List<T: Copy> {
    Zero,
    One([T; 1]),
    Many(Vec<T>),
}

impl<T: Copy> Default for List<T> {
    fn default() -> Self {
        List::Zero
    }
}

impl<T: Copy> List<T> {
    pub(super) fn push(&mut self, x: T) {
        match self {
            List::Zero => *self = List::One([x]),
            List::One([a]) => *self = List::Many(vec![*a, x]),
            List::Many(v) => v.push(x),
        }
    }
}

impl<T: Copy> std::ops::Deref for List<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match self {
            List::Zero => &[],
            List::One(a) => a,
            List::Many(v) => v,
        }
    }
}

impl<T: Copy> std::ops::DerefMut for List<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        match self {
            List::Zero => &mut [],
            List::One(a) => a,
            List::Many(v) => v,
        }
    }
}
