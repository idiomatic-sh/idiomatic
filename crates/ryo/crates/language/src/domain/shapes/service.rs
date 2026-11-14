//! The `shapes` service: free functions named by verb that build, clear and read a file's expression-shape index, with no
//! IO and no ports. The walk is the one copy: tokens are skipped, nodes are visited children first, and the language gives
//! only each node's words.

use super::types::{
    KeptShapeWords, KeyGrowth, ShapeClasses, ShapeExtent, ShapeIndex, ShapeKey, ShapeStep,
};
use crate::domain::diagnostics::types::SyntaxCompleteness;
use crate::domain::elements::service::{end_of, flags_at, has_incomplete_syntax, real_count};
use crate::domain::elements::types::Elements;
use crate::domain::keys::service::{clear_keys, intern_key, key_class_at, push_key_word};
use crate::domain::keys::types::{KeyState, ShapeWord};
use ryo_relations::domain::relations::constants::TOKEN;
use ryo_relations::domain::relations::types::Position;

/// Empties the index for the next file, keeping its capacity.
///
/// # Examples
/// ```
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::shapes::service::{clear_shapes, prepare_shapes};
/// use ryo_language::domain::shapes::types::{ShapeExtent, ShapeIndex, ShapeStep};
/// let mut index = ShapeIndex::default();
/// let elements = Elements::default();
/// assert!(prepare_shapes(&index, &elements, ShapeExtent::Table, |_, _| ShapeStep::Built).is_some());
/// clear_shapes(&mut index);
/// assert_eq!(index, ShapeIndex::default());
/// ```
pub fn clear_shapes(index: &mut ShapeIndex) {
    clear_keys(index.0.get_mut());
}

/// The file's classes, built on the first call after the index was cleared; none when the parser could not read the file
/// completely or a node's key could not be built.
///
/// # Examples
/// ```
/// use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder};
/// use ryo_language::domain::elements::service::kind_at;
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::keys::types::ShapeWord;
/// use ryo_language::domain::shapes::service::{prepare_shapes, push_shape_word, shape_class_at};
/// use ryo_language::domain::shapes::types::{ShapeExtent, ShapeIndex, ShapeStep};
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_relations::domain::relations::types::{KindBits, Offset, Position};
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"ab"));
/// enter_node(&mut builder, KindBits(1), Offset(0));
/// enter_node(&mut builder, KindBits(2), Offset(0));
/// leave_node(&mut builder, Offset(1));
/// enter_node(&mut builder, KindBits(2), Offset(1));
/// leave_node(&mut builder, Offset(2));
/// leave_node(&mut builder, Offset(2));
/// let index = ShapeIndex::default();
/// let classes = prepare_shapes(&index, &elements, ShapeExtent::Table, |at, key| {
///     push_shape_word(key, ShapeWord::Kind(kind_at(&elements, at)));
///     ShapeStep::Built
/// })
/// .expect("every key was built");
/// assert_eq!(shape_class_at(&classes, Position(1)), shape_class_at(&classes, Position(2)));
/// assert_ne!(shape_class_at(&classes, Position(0)), shape_class_at(&classes, Position(1)));
/// ```
pub fn prepare_shapes<'a, Build>(
    index: &'a ShapeIndex,
    elements: &Elements,
    extent: ShapeExtent,
    mut build: Build,
) -> Option<ShapeClasses<'a>>
where
    Build: FnMut(Position, &mut ShapeKey<'_>) -> ShapeStep,
{
    if index.0.borrow().state == KeyState::Fresh {
        let mut held = index.0.borrow_mut();
        if has_incomplete_syntax(elements) == SyntaxCompleteness::Incomplete {
            held.state = KeyState::Partial;
            return None;
        }
        let end = match extent {
            ShapeExtent::Table => end_of(elements).0,
            ShapeExtent::OwnTree => u32::try_from(real_count(elements).0).unwrap_or(u32::MAX),
        };
        let Ok(count) = usize::try_from(end) else {
            held.state = KeyState::Partial;
            return None;
        };
        let missing = count.saturating_sub(held.classes.len());
        if held.classes.try_reserve(missing).is_err() {
            held.state = KeyState::Partial;
            return None;
        }
        held.classes.resize(count, None);
        for raw in (0..count).rev() {
            let Ok(position) = u32::try_from(raw) else {
                held.state = KeyState::Partial;
                continue;
            };
            let at = Position(position);
            if flags_at(elements, at).0 & TOKEN.0 != 0 {
                continue;
            }
            held.scratch.clear();
            let mut key = ShapeKey {
                index: &mut held,
                growth: KeyGrowth::Whole,
            };
            let step = build(at, &mut key);
            let growth = key.growth;
            let class = match (step, growth) {
                (ShapeStep::Skipped, _) => continue,
                (ShapeStep::Unbuilt, _) | (ShapeStep::Built, KeyGrowth::Refused) => {
                    held.state = KeyState::Partial;
                    continue;
                }
                (ShapeStep::SameAs(inner), _) => key_class_at(&held, inner),
                (ShapeStep::Built, KeyGrowth::Whole) => Some(intern_key(&mut held, at)),
            };
            if let Some(slot) = held.classes.get_mut(raw) {
                *slot = class;
            }
        }
        if held.state == KeyState::Fresh {
            held.state = KeyState::Complete;
        }
    }
    let held = index.0.borrow();
    (held.state == KeyState::Complete).then(|| ShapeClasses(held))
}

/// The class of the node at `at`, when it has one.
///
/// # Examples
/// ```
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::shapes::service::{prepare_shapes, shape_class_at};
/// use ryo_language::domain::shapes::types::{ShapeExtent, ShapeIndex, ShapeStep};
/// use ryo_relations::domain::relations::types::Position;
/// let (index, elements) = (ShapeIndex::default(), Elements::default());
/// let classes = prepare_shapes(&index, &elements, ShapeExtent::Table, |_, _| ShapeStep::Built)
///     .expect("an empty file is complete");
/// assert_eq!(shape_class_at(&classes, Position(0)), None);
/// ```
#[must_use]
pub fn shape_class_at(classes: &ShapeClasses<'_>, at: Position) -> Option<Position> {
    key_class_at(&classes.0, at)
}

/// Appends one word to the key; when it cannot be kept, the key is refused and not interned.
///
/// # Examples
/// ```
/// use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder};
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::keys::types::ShapeWord;
/// use ryo_language::domain::shapes::service::{prepare_shapes, push_shape_word, shape_class_at};
/// use ryo_language::domain::shapes::types::{ShapeExtent, ShapeIndex, ShapeStep};
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_relations::domain::relations::types::{KindBits, Offset, Position};
/// use ryo_relations::domain::terms::types::Symbol;
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"a"));
/// enter_node(&mut builder, KindBits(1), Offset(0));
/// leave_node(&mut builder, Offset(1));
/// let index = ShapeIndex::default();
/// let classes = prepare_shapes(&index, &elements, ShapeExtent::Table, |_, key| {
///     push_shape_word(key, ShapeWord::Identifier(Symbol(1)));
///     ShapeStep::Built
/// })
/// .expect("the key was built");
/// assert_eq!(shape_class_at(&classes, Position(0)), Some(Position(0)));
/// ```
pub fn push_shape_word(key: &mut ShapeKey<'_>, word: ShapeWord) {
    if push_key_word(key.index, word).is_none() {
        key.growth = KeyGrowth::Refused;
    }
}

/// The class of a child already built, when it has one.
///
/// # Examples
/// ```
/// use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder};
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::shapes::service::{child_class, prepare_shapes};
/// use ryo_language::domain::shapes::types::{ShapeExtent, ShapeIndex, ShapeStep};
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_relations::domain::relations::types::{KindBits, Offset, Position};
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"a"));
/// enter_node(&mut builder, KindBits(1), Offset(0));
/// enter_node(&mut builder, KindBits(2), Offset(0));
/// leave_node(&mut builder, Offset(1));
/// leave_node(&mut builder, Offset(1));
/// let index = ShapeIndex::default();
/// let mut seen = Vec::new();
/// let _classes = prepare_shapes(&index, &elements, ShapeExtent::Table, |_, key| {
///     seen.push(child_class(key, Position(1)));
///     ShapeStep::Built
/// });
/// assert_eq!(seen, [None, Some(Position(1))]);
/// ```
#[must_use]
pub fn child_class(key: &ShapeKey<'_>, child: Position) -> Option<Position> {
    key_class_at(key.index, child)
}

/// Rewrites the key's words in place: `compact` keeps the first of them and answers how many.
///
/// # Examples
/// ```
/// use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder};
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::keys::types::ShapeWord;
/// use ryo_language::domain::shapes::service::{
///     prepare_shapes, push_shape_word, rewrite_shape_words, shape_class_at,
/// };
/// use ryo_language::domain::shapes::types::{KeptShapeWords, ShapeExtent, ShapeIndex, ShapeStep};
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_relations::domain::relations::types::{KindBits, Offset, Position};
/// use ryo_relations::domain::terms::types::Symbol;
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"ab"));
/// enter_node(&mut builder, KindBits(1), Offset(0));
/// leave_node(&mut builder, Offset(1));
/// enter_node(&mut builder, KindBits(1), Offset(1));
/// leave_node(&mut builder, Offset(2));
/// let index = ShapeIndex::default();
/// let classes = prepare_shapes(&index, &elements, ShapeExtent::Table, |at, key| {
///     push_shape_word(key, ShapeWord::Literal(Symbol(1)));
///     if at == Position(1) {
///         push_shape_word(key, ShapeWord::Literal(Symbol(2)));
///         rewrite_shape_words(key, |_words| KeptShapeWords(1));
///     }
///     ShapeStep::Built
/// })
/// .expect("every key was built");
/// assert_eq!(shape_class_at(&classes, Position(0)), Some(Position(1)));
/// ```
pub fn rewrite_shape_words<Compact>(key: &mut ShapeKey<'_>, compact: Compact)
where
    Compact: FnOnce(&mut [ShapeWord]) -> KeptShapeWords,
{
    let kept = compact(key.index.scratch.as_mut_slice());
    key.index.scratch.truncate(kept.0);
}
