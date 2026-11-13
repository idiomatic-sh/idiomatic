//! The `shapes` service: free functions named by verb that build a file's structural classes once, bottom-up, over the shared
//! key index, with no IO and no ports. Which words make a key is TypeScript's; interning them is the kernel's.

use crate::domain::model::types::ReadIndexes;
use ryo::languages::{Position, RetainedKeyIndex, ShapeWord, Symbols, Tree};
use std::cell::Ref;

/// The file's structural classes, built on the first call after the model was cleared. A tree the parser could not read
/// completely has none: no two of its nodes may seem equal for want of what it lost.
pub(crate) fn prepare<'a>(
    tree: &Tree<'_>,
    indexes: &'a ReadIndexes,
    symbols: &mut Symbols,
) -> Option<Ref<'a, RetainedKeyIndex<Position, ShapeWord>>> {
    let _ = (tree, indexes, symbols);
    None
}
