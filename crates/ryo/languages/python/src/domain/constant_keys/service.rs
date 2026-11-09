//! Lazily build full, parser-derived constant keys for supported Python expressions.

use crate::domain::constant_keys::types::RetainedKeyIndex;
use crate::domain::constant_keys::types::{
    BuiltKey, KeyClass, KeyHash, KeyRecord, KeyRecordIndex, KeyState, KeyVisibility, KeyWord,
    LiteralKind,
};
use crate::domain::constant_keys::types::{ConstantKeyFragment, KeyWordIndex, WordRange};
use crate::domain::{
    constant_numbers::{
        service::{float_key, negate, number, property, to_float},
        types::{ExactInteger, NumberKey, NumericState, RealKey},
    },
    fields::service::field_of,
    labels::types::Field,
    nodes::types::Node,
    syntax::{
        constants::{
            BYTES_SCALAR, COMPLEX_SCALAR, ELLIPSIS_SCALAR, INTEGER_SCALAR, NONE_SCALAR,
            SCALAR_KIND, SCALAR_VALUE,
        },
        service::{below, kind, literal},
        types::{Kind, Literal},
    },
    tokens::types::Token,
};
use ryo::languages::{self, Position, Symbols, Tree};
use std::cell::{Ref, RefCell};
use std::hash::{Hash, Hasher};

/// Read the already-built numeric payload at a child expression.
fn numeric_class(classes: &[Option<KeyClass>], at: Position) -> Option<NumericState> {
    let Ok(slot) = usize::try_from(at.0) else {
        return None;
    };
    classes.get(slot)?.as_ref()?.number
}

/// Invalidate file-local keys while retaining reusable allocations.
pub(crate) fn clear(index: &mut RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>) {
    index.classes.clear();
    index.words.clear();
    index.records.clear();
    index.buckets.clear();
    index.scratch.clear();
    index.state = KeyState::Fresh;
}

/// The retained representative for one supported occurrence.
#[must_use]
pub(crate) fn key_of(
    index: &RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>,
    at: Position,
) -> Option<Position> {
    let Ok(slot) = usize::try_from(at.0) else {
        return None;
    };
    index
        .classes
        .get(slot)
        .copied()
        .flatten()
        .map(|class| class.representative)
}

/// Find a child in the requested Python AST field.
fn child_at(tree: &Tree<'_>, at: Position, wanted: Field) -> Option<Position> {
    languages::children(tree.elements, at).find(|child| field_of(tree, *child) == wanted)
}

/// Build a structural key for a nonnumeric literal from its parser payload.
fn literal_word(tree: &Tree<'_>, at: Position, symbols: &mut Symbols) -> Option<KeyWord> {
    let scalar = property(tree, at, SCALAR_KIND)?.0;
    let (category, value) = match scalar {
        BYTES_SCALAR => (LiteralKind::Bytes, property(tree, at, SCALAR_VALUE)?.0),
        INTEGER_SCALAR => (LiteralKind::Integer, property(tree, at, SCALAR_VALUE)?.0),
        NONE_SCALAR => return Some(KeyWord::Literal(LiteralKind::Null, None)),
        ELLIPSIS_SCALAR => return Some(KeyWord::Literal(LiteralKind::Ellipsis, None)),
        _ => return None,
    };
    Some(KeyWord::Literal(
        category,
        Some(languages::intern_derived(
            symbols,
            tree.source,
            languages::NameText(value),
        )),
    ))
}

/// Append one key component, marking the reader partial if its retained buffer cannot grow.
fn push_word(
    index: &mut RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>,
    word: KeyWord,
) -> Option<()> {
    if index.scratch.try_reserve(1).is_err() {
        index.state = KeyState::Partial;
        return None;
    }
    index.scratch.push(word);
    Some(())
}

/// Whether an occurrence is a synthetic literal segment inside an interpolated string.
fn interpolation_fragment(tree: &Tree<'_>, at: Position) -> ConstantKeyFragment {
    use crate::domain::syntax::service::parent;
    let mut child = at;
    let mut above = parent(tree, child);
    while above != child {
        match kind(tree, above) {
            Kind::Node(Node::FormattedValue | Node::Interpolation) => {
                return ConstantKeyFragment::Standalone;
            }
            Kind::Node(Node::JoinedStr | Node::TemplateStr) => {
                return ConstantKeyFragment::Interpolated;
            }
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
        child = above;
        above = parent(tree, above);
    }
    ConstantKeyFragment::Standalone
}

/// Build a key for a primitive constant, retaining decoded strings and bounded numeric values.
fn constant_key(
    tree: &Tree<'_>,
    at: Position,
    symbols: &mut Symbols,
    index: &mut RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>,
) -> Option<BuiltKey> {
    if interpolation_fragment(tree, at) == ConstantKeyFragment::Interpolated {
        return None;
    }
    if property(tree, at, SCALAR_KIND).map(|text| text.0) == Some(INTEGER_SCALAR)
        && property(tree, at, SCALAR_VALUE).is_some_and(|value| value.0.parse::<u64>().is_err())
    {
        push_word(index, literal_word(tree, at, symbols)?)?;
        return Some(BuiltKey {
            number: Some(NumericState::StructuralInteger),
        });
    }
    if let Some(number) = number(tree, at) {
        push_word(index, KeyWord::Number(number))?;
        return Some(BuiltKey {
            number: Some(NumericState::Number(number)),
        });
    }
    if literal(tree, at) == Literal::String {
        let begin = languages::begin_name(symbols);
        let mut found = false;
        for token in
            below(tree, at).filter(|position| kind(tree, *position) == Kind::Token(Token::String))
        {
            languages::push_text(symbols, languages::decoded_at(tree.elements, token)?);
            found = true;
        }
        if !found {
            return None;
        }
        push_word(
            index,
            KeyWord::Literal(
                LiteralKind::String,
                Some(languages::end_name(symbols, tree.source, begin)),
            ),
        )?;
        return Some(BuiltKey { number: None });
    }
    push_word(index, literal_word(tree, at, symbols)?)?;
    Some(BuiltKey { number: None })
}

/// Combine already-built child keys in tuple order.
fn tuple_key(
    tree: &Tree<'_>,
    at: Position,
    index: &mut RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>,
) -> Option<BuiltKey> {
    push_word(index, KeyWord::Tuple)?;
    for child in
        languages::children(tree.elements, at).filter(|child| field_of(tree, *child) == Field::Elts)
    {
        let Ok(slot) = usize::try_from(child.0) else {
            return None;
        };
        let Some(Some(class)) = index.classes.get(slot).copied() else {
            return None;
        };
        push_word(index, KeyWord::Child(class.representative))?;
    }
    push_word(index, KeyWord::EndTuple)?;
    Some(BuiltKey { number: None })
}

/// Apply unary plus or minus to a supported numeric child.
fn unary_key(
    tree: &Tree<'_>,
    at: Position,
    index: &mut RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>,
) -> Option<BuiltKey> {
    let operand = child_at(tree, at, Field::Operand)?;
    let NumericState::Number(mut number) = numeric_class(&index.classes, operand)? else {
        // Oversized integer signs stay on the ordinary structural-shape fallback path.
        return None;
    };
    let operator =
        languages::children(tree.elements, at).find_map(|child| match kind(tree, child) {
            Kind::Token(token @ (Token::Plus | Token::Minus)) => Some(token),
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => None,
        })?;
    if operator == Token::Minus {
        number.real = negate(number.real)?;
        number.imaginary = negate(number.imaginary)?;
    }
    push_word(index, KeyWord::Number(number))?;
    Some(BuiltKey {
        number: Some(NumericState::Number(number)),
    })
}

/// Normalize an approved real-plus-or-minus-complex literal expression.
fn complex_binary_key(
    tree: &Tree<'_>,
    at: Position,
    index: &mut RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>,
) -> Option<BuiltKey> {
    let left = child_at(tree, at, Field::Left)?;
    let right = child_at(tree, at, Field::Right)?;
    if property(tree, right, SCALAR_KIND).map(|text| text.0) != Some(COMPLEX_SCALAR) {
        return None;
    }
    let operator =
        languages::children(tree.elements, at).find_map(|child| match kind(tree, child) {
            Kind::Token(token @ (Token::Plus | Token::Minus)) => Some(token),
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => None,
        })?;
    let NumericState::Number(left_number) = numeric_class(&index.classes, left)? else {
        return None;
    };
    let NumericState::Number(right_number) = numeric_class(&index.classes, right)? else {
        return None;
    };
    if left_number.imaginary != RealKey::Integer(ExactInteger(0))
        || right_number.real != RealKey::Integer(ExactInteger(0))
    {
        return None;
    }
    let left_real = to_float(left_number.real);
    let mut right_imaginary = to_float(right_number.imaginary);
    if operator == Token::Minus {
        right_imaginary = -right_imaginary;
    }
    let result = NumberKey {
        real: float_key(left_real),
        imaginary: float_key(right_imaginary),
    };
    push_word(index, KeyWord::Number(result))?;
    Some(BuiltKey {
        number: Some(NumericState::Number(result)),
    })
}

/// Dispatch an AST occurrence to its one supported key constructor.
fn build_key(
    tree: &Tree<'_>,
    at: Position,
    symbols: &mut Symbols,
    index: &mut RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>,
) -> Option<BuiltKey> {
    index.scratch.clear();
    match kind(tree, at) {
        Kind::Node(Node::Constant) => constant_key(tree, at, symbols, index),
        Kind::Node(Node::Tuple) => tuple_key(tree, at, index),
        Kind::Node(Node::UnaryOp) => unary_key(tree, at, index),
        Kind::Node(Node::BinOp) => complex_binary_key(tree, at, index),
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => None,
    }
}

fn intern(
    index: &mut RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>,
    at: Position,
) -> Option<Position> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    Hash::hash(&index.scratch, &mut hasher);
    intern_with_hash(index, at, KeyHash(Hasher::finish(&hasher)))
}

/// Intern after a caller-selected hash, also making collision handling directly testable.
pub(super) fn intern_with_hash(
    index: &mut RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>,
    at: Position,
    hash: KeyHash,
) -> Option<Position> {
    let mut candidate = index.buckets.get(&hash).copied();
    while let Some(place) = candidate {
        let record = index.records.get(place.0)?;
        if index.words.get(record.words.start.0..record.words.end.0)
            == Some(index.scratch.as_slice())
        {
            return Some(record.representative);
        }
        candidate = record.previous;
    }
    if index.words.try_reserve(index.scratch.len()).is_err()
        || index.records.try_reserve(1).is_err()
        || index.buckets.try_reserve(1).is_err()
    {
        return None;
    }
    let start = index.words.len();
    index.words.extend_from_slice(&index.scratch);
    let place = KeyRecordIndex(index.records.len());
    index.records.push(KeyRecord {
        words: WordRange {
            start: KeyWordIndex(start),
            end: KeyWordIndex(index.words.len()),
        },
        representative: at,
        previous: index.buckets.get(&hash).copied(),
    });
    let _previous_bucket_head = index.buckets.insert(hash, place);
    Some(at)
}

/// Prepare supported classes once, retaining complete rows even when another occurrence is unknown.
/// Candidate visibility comes from the caller's semantic projection.
pub(crate) fn prepare<'a>(
    tree: &Tree<'_>,
    held: &'a RefCell<RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>>,
    symbols: &mut Symbols,
    mut visible: impl FnMut(Position, &mut Symbols) -> KeyVisibility,
) -> Ref<'a, RetainedKeyIndex<KeyClass, KeyWord, KeyHash, KeyState>> {
    if held.borrow().state == KeyState::Fresh {
        let mut index = held.borrow_mut();
        let count = usize::try_from(languages::end_of(tree.elements).0).unwrap_or(usize::MAX);
        if index.classes.try_reserve(count).is_err() {
            index.state = KeyState::Partial;
            drop(index);
            return held.borrow();
        }
        index.classes.resize(count, None);
        if languages::has_incomplete_syntax(tree.elements)
            == languages::SyntaxCompleteness::Incomplete
        {
            index.state = KeyState::Partial;
        }
        for raw in (0..count).rev() {
            let Ok(position) = u32::try_from(raw) else {
                index.state = KeyState::Partial;
                continue;
            };
            let at = Position(position);
            if visible(at, symbols) != KeyVisibility::Visible {
                continue;
            }
            if !matches!(
                kind(tree, at),
                Kind::Node(Node::Constant | Node::Tuple | Node::UnaryOp | Node::BinOp)
            ) {
                continue;
            }
            match build_key(tree, at, symbols, &mut index) {
                Some(built) => match intern(&mut index, at) {
                    Some(representative) => {
                        if let Some(slot) = index.classes.get_mut(raw) {
                            *slot = Some(KeyClass {
                                representative,
                                number: built.number,
                            });
                        }
                    }
                    None => index.state = KeyState::Partial,
                },
                None => {
                    // Only actual primitive nodes with missing parser values make absence uncertain.
                    if kind(tree, at) == Kind::Node(Node::Constant)
                        && interpolation_fragment(tree, at) == ConstantKeyFragment::Standalone
                        && (literal(tree, at) != Literal::Other
                            || property(tree, at, SCALAR_KIND).is_some())
                    {
                        index.state = KeyState::Partial;
                    }
                }
            }
        }
        if index.state == KeyState::Fresh {
            index.state = KeyState::Complete;
        }
    }
    held.borrow()
}
