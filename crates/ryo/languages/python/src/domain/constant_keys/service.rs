//! Lazily build full, parser-derived constant keys for supported Python expressions.

use crate::domain::constant_keys::types::{
    ConstantKeyIndex, KeyClass, KeyRecord, KeyRecordIndex, KeyState, KeyWord, LiteralKind,
    NumberKey, NumericState, RealKey,
};
use crate::domain::{
    fields::service::field_of,
    labels::types::Field,
    nodes::types::Node,
    syntax::{
        constants::{
            BOOLEAN_SCALAR, BYTES_SCALAR, COMPLEX_SCALAR, ELLIPSIS_SCALAR, FLOAT_SCALAR,
            INTEGER_SCALAR, NONE_SCALAR, SCALAR_IMAGINARY, SCALAR_KIND, SCALAR_VALUE,
        },
        service::{below, kind, literal},
        types::{Kind, Literal},
    },
    tokens::types::Token,
};
use ryo::languages::{self, Position, Symbols, Tree};
use std::cell::Ref;
use std::hash::{Hash, Hasher};

/// Invalidate file-local keys while retaining reusable allocations.
pub(crate) fn clear(index: &mut ConstantKeyIndex) {
    index.classes.clear();
    index.words.clear();
    index.records.clear();
    index.buckets.clear();
    index.scratch.clear();
    index.state = KeyState::Fresh;
}

/// The retained representative for one supported occurrence.
#[must_use]
pub(crate) fn key_of(index: &ConstantKeyIndex, at: Position) -> Option<Position> {
    index
        .classes
        .get(usize::try_from(at.0).ok()?)
        .copied()
        .flatten()
        .map(|class| class.representative)
}

fn property<'a>(tree: &'a Tree<'_>, at: Position, id: languages::PropertyId) -> Option<&'a str> {
    languages::property_at(tree.elements, at, id).map(|value| value.0)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "the upper bound and exact round trip prove this value is an exact supported integer"
)]
fn float_key(value: f64) -> RealKey {
    if value == 0.0 {
        return RealKey::Integer(0);
    }
    if value.is_finite() && value.abs() < 18_446_744_073_709_551_616.0 {
        let integer = value as i128;
        if integer as f64 == value {
            return RealKey::Integer(integer);
        }
    }
    RealKey::Float(value.to_bits())
}

fn number(tree: &Tree<'_>, at: Position) -> Option<NumberKey> {
    let scalar = property(tree, at, SCALAR_KIND)?;
    let zero = RealKey::Integer(0);
    let answer = match scalar {
        INTEGER_SCALAR => match property(tree, at, SCALAR_VALUE)?.parse::<u64>() {
            Ok(value) => NumberKey {
                real: RealKey::Integer(i128::from(value)),
                imaginary: zero,
            },
            Err(_) => return None,
        },
        FLOAT_SCALAR => NumberKey {
            real: float_key(f64::from_bits(
                property(tree, at, SCALAR_VALUE)?.parse().ok()?,
            )),
            imaginary: zero,
        },
        COMPLEX_SCALAR => NumberKey {
            real: float_key(f64::from_bits(
                property(tree, at, SCALAR_VALUE)?.parse().ok()?,
            )),
            imaginary: float_key(f64::from_bits(
                property(tree, at, SCALAR_IMAGINARY)?.parse().ok()?,
            )),
        },
        BOOLEAN_SCALAR => NumberKey {
            real: RealKey::Integer(if property(tree, at, SCALAR_VALUE)?.parse::<bool>().ok()? {
                1
            } else {
                0
            }),
            imaginary: zero,
        },
        _ => return None,
    };
    Some(answer)
}

fn numeric_class(classes: &[Option<KeyClass>], at: Position) -> Option<NumericState> {
    classes.get(usize::try_from(at.0).ok()?)?.as_ref()?.number
}

fn child_at(tree: &Tree<'_>, at: Position, wanted: Field) -> Option<Position> {
    languages::children(tree.elements, at).find(|child| field_of(tree, *child) == wanted)
}

fn literal_word(tree: &Tree<'_>, at: Position, symbols: &mut Symbols) -> Option<KeyWord> {
    let scalar = property(tree, at, SCALAR_KIND)?;
    let (category, value) = match scalar {
        BYTES_SCALAR => (LiteralKind::Bytes, property(tree, at, SCALAR_VALUE)?),
        INTEGER_SCALAR => (LiteralKind::Integer, property(tree, at, SCALAR_VALUE)?),
        FLOAT_SCALAR | COMPLEX_SCALAR | BOOLEAN_SCALAR => return None,
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

fn push_word(index: &mut ConstantKeyIndex, word: KeyWord) -> Option<()> {
    if index.scratch.try_reserve(1).is_err() {
        index.state = KeyState::Partial;
        return None;
    }
    index.scratch.push(word);
    Some(())
}

fn interpolation_fragment(tree: &Tree<'_>, at: Position) -> bool {
    use crate::domain::syntax::service::parent;
    let mut child = at;
    let mut above = parent(tree, child);
    while above != child {
        match kind(tree, above) {
            Kind::Node(Node::FormattedValue | Node::Interpolation) => return false,
            Kind::Node(Node::JoinedStr | Node::TemplateStr) => return true,
            _ => {}
        }
        child = above;
        above = parent(tree, above);
    }
    false
}

fn constant_key(
    tree: &Tree<'_>,
    at: Position,
    symbols: &mut Symbols,
    index: &mut ConstantKeyIndex,
) -> Option<Option<NumericState>> {
    if interpolation_fragment(tree, at) {
        return None;
    }
    if property(tree, at, SCALAR_KIND) == Some(INTEGER_SCALAR)
        && property(tree, at, SCALAR_VALUE).is_some_and(|value| value.parse::<u64>().is_err())
    {
        push_word(index, literal_word(tree, at, symbols)?)?;
        return Some(Some(NumericState::StructuralInteger));
    }
    if let Some(number) = number(tree, at) {
        push_word(index, KeyWord::Number(number))?;
        return Some(Some(NumericState::Number(number)));
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
        return Some(None);
    }
    push_word(index, literal_word(tree, at, symbols)?)?;
    Some(None)
}

fn tuple_key(
    tree: &Tree<'_>,
    at: Position,
    index: &mut ConstantKeyIndex,
) -> Option<Option<NumericState>> {
    push_word(index, KeyWord::Tuple)?;
    for child in
        languages::children(tree.elements, at).filter(|child| field_of(tree, *child) == Field::Elts)
    {
        let Some(Some(class)) = index.classes.get(usize::try_from(child.0).ok()?).copied() else {
            return None;
        };
        push_word(index, KeyWord::Child(class.representative))?;
    }
    push_word(index, KeyWord::EndTuple)?;
    Some(None)
}

fn unary_key(
    tree: &Tree<'_>,
    at: Position,
    index: &mut ConstantKeyIndex,
) -> Option<Option<NumericState>> {
    let operand = child_at(tree, at, Field::Operand)?;
    let NumericState::Number(mut number) = numeric_class(&index.classes, operand)? else {
        // Oversized integer signs stay on the ordinary structural-shape fallback path.
        return None;
    };
    let operator =
        languages::children(tree.elements, at).find_map(|child| match kind(tree, child) {
            Kind::Token(token @ (Token::Plus | Token::Minus)) => Some(token),
            _ => None,
        })?;
    if operator == Token::Minus {
        number.real = negate(number.real)?;
        number.imaginary = negate(number.imaginary)?;
    }
    push_word(index, KeyWord::Number(number))?;
    Some(Some(NumericState::Number(number)))
}

fn complex_binary_key(
    tree: &Tree<'_>,
    at: Position,
    index: &mut ConstantKeyIndex,
) -> Option<Option<NumericState>> {
    let left = child_at(tree, at, Field::Left)?;
    let right = child_at(tree, at, Field::Right)?;
    if property(tree, right, SCALAR_KIND) != Some(COMPLEX_SCALAR) {
        return None;
    }
    let operator =
        languages::children(tree.elements, at).find_map(|child| match kind(tree, child) {
            Kind::Token(token @ (Token::Plus | Token::Minus)) => Some(token),
            _ => None,
        })?;
    let NumericState::Number(left_number) = numeric_class(&index.classes, left)? else {
        return None;
    };
    let NumericState::Number(right_number) = numeric_class(&index.classes, right)? else {
        return None;
    };
    if left_number.imaginary != RealKey::Integer(0) || right_number.real != RealKey::Integer(0) {
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
    Some(Some(NumericState::Number(result)))
}

fn build_key(
    tree: &Tree<'_>,
    at: Position,
    symbols: &mut Symbols,
    index: &mut ConstantKeyIndex,
) -> Option<Option<NumericState>> {
    index.scratch.clear();
    match kind(tree, at) {
        Kind::Node(Node::Constant) => constant_key(tree, at, symbols, index),
        Kind::Node(Node::Tuple) => tuple_key(tree, at, index),
        Kind::Node(Node::UnaryOp) => unary_key(tree, at, index),
        Kind::Node(Node::BinOp) => complex_binary_key(tree, at, index),
        _ => None,
    }
}

fn negate(value: RealKey) -> Option<RealKey> {
    match value {
        RealKey::Integer(value) => value.checked_neg().map(RealKey::Integer),
        RealKey::Float(bits) => Some(RealKey::Float((-f64::from_bits(bits)).to_bits())),
    }
}
fn to_float(value: RealKey) -> f64 {
    match value {
        RealKey::Integer(value) => integer_as_float(value),
        RealKey::Float(bits) => f64::from_bits(bits),
    }
}
#[expect(
    clippy::cast_precision_loss,
    reason = "Python complex arithmetic converts the real operand to f64 before composing the result"
)]
fn integer_as_float(value: i128) -> f64 {
    value as f64
}
fn intern(index: &mut ConstantKeyIndex, at: Position) -> Option<Position> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    index.scratch.hash(&mut hasher);
    intern_with_hash(index, at, hasher.finish())
}

/// Intern after a caller-selected hash, also making collision handling directly testable.
pub(super) fn intern_with_hash(
    index: &mut ConstantKeyIndex,
    at: Position,
    hash: u64,
) -> Option<Position> {
    let mut candidate = index.buckets.get(&hash).copied();
    while let Some(place) = candidate {
        let record = index.records.get(place.0)?;
        if index.words.get(record.words.clone()) == Some(index.scratch.as_slice()) {
            return Some(record.representative);
        }
        candidate = record.previous;
    }
    index.words.try_reserve(index.scratch.len()).ok()?;
    index.records.try_reserve(1).ok()?;
    index.buckets.try_reserve(1).ok()?;
    let start = index.words.len();
    index.words.extend_from_slice(&index.scratch);
    let place = KeyRecordIndex(index.records.len());
    index.records.push(KeyRecord {
        words: start..index.words.len(),
        representative: at,
        previous: index.buckets.get(&hash).copied(),
    });
    let _ = index.buckets.insert(hash, place);
    Some(at)
}

/// Prepare supported classes once, retaining complete rows even when another occurrence is unknown.
pub(crate) fn prepare<'a>(
    tree: &Tree<'_>,
    model: &'a crate::domain::model::types::Declarations,
    symbols: &mut Symbols,
) -> Ref<'a, ConstantKeyIndex> {
    let held = &model.constant_keys;
    if held.borrow().state == KeyState::Fresh {
        let mut index = held.borrow_mut();
        let count = usize::try_from(languages::end_of(tree.elements).0).unwrap_or(usize::MAX);
        if index.classes.try_reserve(count).is_err() {
            index.state = KeyState::Partial;
            drop(index);
            return held.borrow();
        }
        index.classes.resize(count, None);
        if languages::has_incomplete_syntax(tree.elements) {
            index.state = KeyState::Partial;
        }
        for raw in (0..count).rev() {
            let Ok(position) = u32::try_from(raw) else {
                index.state = KeyState::Partial;
                continue;
            };
            let at = Position(position);
            if crate::domain::semantic_syntax::service::visibility(tree, model, at, symbols)
                != crate::domain::semantic_syntax::types::Visibility::Visible
            {
                continue;
            }
            if !matches!(
                kind(tree, at),
                Kind::Node(Node::Constant | Node::Tuple | Node::UnaryOp | Node::BinOp)
            ) {
                continue;
            }
            match build_key(tree, at, symbols, &mut index) {
                Some(number) => match intern(&mut index, at) {
                    Some(representative) => {
                        if let Some(slot) = index.classes.get_mut(raw) {
                            *slot = Some(KeyClass {
                                representative,
                                number,
                            });
                        }
                    }
                    None => index.state = KeyState::Partial,
                },
                None => {
                    // Only actual primitive nodes with missing parser values make absence uncertain.
                    if kind(tree, at) == Kind::Node(Node::Constant)
                        && !interpolation_fragment(tree, at)
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
