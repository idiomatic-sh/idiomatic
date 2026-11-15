//! The types of the `shapes` concept: a file's expression-shape index, the classes read from it, the key of one node being
//! built, and what a language answers for each node. The index's state is private: only this concept's service builds,
//! clears and reads it.

use crate::domain::keys::types::{RetainedKeyIndex, ShapeWord};
use ryo_relations::domain::relations::types::Position;
use std::cell::{Ref, RefCell};

/// A file's structural classes, built on the first read after it was cleared and kept with its capacity between files. A
/// language holds it and never reaches its keys: it cannot push a word, clear it or read a class but through the service.
///
/// # Examples
/// ```compile_fail,E0616
/// use ryo_language::domain::keys::types::ShapeWord;
/// use ryo_language::domain::shapes::types::ShapeIndex;
/// use ryo_relations::domain::terms::types::Symbol;
/// let index = ShapeIndex::default();
/// index.0.borrow_mut().scratch.push(ShapeWord::Identifier(Symbol(1)));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShapeIndex(pub(super) RefCell<RetainedKeyIndex<Position, ShapeWord>>);

/// The classes of a complete index, borrowed for reading.
///
/// # Examples
/// ```compile_fail,E0616
/// use ryo_language::domain::shapes::types::ShapeClasses;
/// fn classes(read: &ShapeClasses<'_>) -> usize {
///     read.0.classes.len()
/// }
/// ```
#[derive(Debug)]
pub struct ShapeClasses<'a>(pub(super) Ref<'a, RetainedKeyIndex<Position, ShapeWord>>);

/// Whether every word pushed to a key was kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum KeyGrowth {
    /// Every word was kept.
    Whole,
    /// A word could not be kept: the key is not interned.
    Refused,
}

/// The key of one node being built: a language appends its words and reads its children's classes, nothing else.
///
/// # Examples
/// ```compile_fail,E0616
/// use ryo_language::domain::shapes::types::ShapeKey;
/// fn clear(key: &mut ShapeKey<'_>) {
///     key.index.scratch.clear();
/// }
/// ```
#[derive(Debug)]
pub struct ShapeKey<'a> {
    /// The index whose scratch holds the key.
    pub(super) index: &'a mut RetainedKeyIndex<Position, ShapeWord>,
    /// Whether every word pushed was kept.
    pub(super) growth: KeyGrowth,
}

/// How many words a rewrite keeps, from the first.
///
/// # Examples
/// ```
/// use ryo_language::domain::shapes::types::KeptShapeWords;
/// assert_eq!(KeptShapeWords(2).0, 2);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeptShapeWords(pub usize);

/// What a language answers for one node, children first.
///
/// # Examples
/// ```
/// use ryo_language::domain::shapes::types::ShapeStep;
/// use ryo_relations::domain::relations::types::Position;
/// assert_ne!(ShapeStep::SameAs(Position(1)), ShapeStep::Built);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum ShapeStep {
    /// The node takes no class and leaves the index complete.
    Skipped,
    /// The node's key could not be built: it takes no class and the index is partial.
    Unbuilt,
    /// The node takes the class of the node at the position, already built.
    SameAs(Position),
    /// The key holds the node's every word: it is interned.
    Built,
}

/// Which elements of the file take a class.
///
/// # Examples
/// ```
/// use ryo_language::domain::shapes::types::ShapeExtent;
/// assert_ne!(ShapeExtent::Table, ShapeExtent::OwnTree);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeExtent {
    /// Every element of the table, those the language added for reading included.
    Table,
    /// The file's own tree, without the elements the language added for reading.
    OwnTree,
}
