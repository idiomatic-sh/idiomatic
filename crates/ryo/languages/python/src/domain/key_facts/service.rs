//! The `key_facts` service: free functions over the parser's own constant expressions. Two constants have one key when Python
//! holds them equal as dictionary keys: numbers by exact value across `bool`, `int`, `float` and `complex`, a signed number and
//! `real ± imaginary` folded, strings by their decoded value, bytes, `None`, `...`, and tuples of keyed elements. The key words,
//! the number keys and the classes are the `constant_keys` and `constant_numbers` concepts'; the keys are interned exactly.

use super::types::KeyBuild;
use crate::domain::constant_keys::types::{KeyClass, KeyWord, LiteralKind};
use crate::domain::constant_numbers::service::{float_key, negate, to_float};
use crate::domain::constant_numbers::types::{
    Binary64, ExactInteger, NumberKey, NumericState, RealKey,
};
use crate::domain::node::service::{id_of, word};
use crate::domain::node::types::SyntaxFile;
use ruff_python_ast::{AnyNodeRef, Expr, ExprBinOp, ExprUnaryOp, Number, Operator, UnaryOp};
use ryo::languages::{NameText, Position, RetainedKeyIndex, clear_keys, intern_key, push_key_word};
use std::fmt::Write as _;

/// The class of an operand, when it has a key.
fn class_of(index: &RetainedKeyIndex<KeyClass, KeyWord>, operand: &Expr) -> Option<KeyClass> {
    index
        .classes
        .get(usize::from(id_of(operand)?))
        .copied()
        .flatten()
}

/// The number an operand stands for, when it is one of exact value.
fn number_of(index: &RetainedKeyIndex<KeyClass, KeyWord>, operand: &Expr) -> Option<NumberKey> {
    match class_of(index, operand)?.number? {
        NumericState::Number(number) => Some(number),
        NumericState::StructuralInteger => None,
    }
}

/// A real number with no imaginary part.
const fn real(value: RealKey) -> NumberKey {
    NumberKey {
        real: value,
        imaginary: RealKey::Integer(ExactInteger(0)),
    }
}

/// Writes the key of a number and answers it.
fn numbered(build: &mut KeyBuild<'_>, number: NumberKey) -> Option<Option<NumericState>> {
    push_key_word(build.index, KeyWord::Number(number))?;
    Some(Some(NumericState::Number(number)))
}

/// Writes the key of a literal that is equal only to itself: its category and its value as a word of the file.
fn literal(build: &mut KeyBuild<'_>, kind: LiteralKind, value: Option<NameText<'_>>) -> Option<()> {
    let value = value.map(|text| word(build.words, text));
    push_key_word(build.index, KeyWord::Literal(kind, value))
}

/// Writes the key of a signed number: the operand's number, negated under a minus.
fn signed(build: &mut KeyBuild<'_>, operation: &ExprUnaryOp) -> Option<Option<NumericState>> {
    let mut number = number_of(build.index, &operation.operand)?;
    match operation.op {
        UnaryOp::UAdd => {}
        UnaryOp::USub => {
            number.real = negate(number.real)?;
            number.imaginary = negate(number.imaginary)?;
        }
        UnaryOp::Invert | UnaryOp::Not => return None,
    }
    numbered(build, number)
}

/// Writes the key of `real ± imaginary`: a real number and an imaginary literal folded into one complex number.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only a sum and a difference fold a real and an imaginary literal; no other operator makes a constant key"
)]
fn folded(build: &mut KeyBuild<'_>, operation: &ExprBinOp) -> Option<Option<NumericState>> {
    let Expr::NumberLiteral(right) = &*operation.right else {
        return None;
    };
    if !matches!(right.value, Number::Complex { .. }) {
        return None;
    }
    let minus = match operation.op {
        Operator::Add => false,
        Operator::Sub => true,
        _ => return None,
    };
    let left = number_of(build.index, &operation.left)?;
    let right = number_of(build.index, &operation.right)?;
    let zero = RealKey::Integer(ExactInteger(0));
    if left.imaginary != zero || right.real != zero {
        return None;
    }
    let imaginary = to_float(right.imaginary);
    numbered(
        build,
        NumberKey {
            real: float_key(to_float(left.real)),
            imaginary: float_key(if minus { -imaginary } else { imaginary }),
        },
    )
}

/// Writes the key of `node` into the key in hand and answers the number it stands for, if any; none for a node that is no
/// constant Python compares by value. Its operands' classes are read from the classes found so far.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the constants Python compares by value are named; every other node has no key"
)]
fn key_words(build: &mut KeyBuild<'_>, node: AnyNodeRef<'_>) -> Option<Option<NumericState>> {
    match node {
        AnyNodeRef::ExprNumberLiteral(literal_number) => match &literal_number.value {
            Number::Int(integer) => {
                if let Some(value) = integer.as_u64() {
                    numbered(
                        build,
                        real(RealKey::Integer(ExactInteger(i128::from(value)))),
                    )
                } else {
                    build.text.0.clear();
                    // Writing into a `String` does not fail.
                    let _written: std::fmt::Result = write!(build.text.0, "{integer}");
                    let value = word(build.words, NameText(&build.text.0));
                    push_key_word(
                        build.index,
                        KeyWord::Literal(LiteralKind::Integer, Some(value)),
                    )?;
                    Some(Some(NumericState::StructuralInteger))
                }
            }
            Number::Float(value) => numbered(build, real(float_key(Binary64(*value)))),
            Number::Complex { real, imag } => numbered(
                build,
                NumberKey {
                    real: float_key(Binary64(*real)),
                    imaginary: float_key(Binary64(*imag)),
                },
            ),
        },
        AnyNodeRef::ExprBooleanLiteral(boolean) => numbered(
            build,
            real(RealKey::Integer(ExactInteger(i128::from(boolean.value)))),
        ),
        AnyNodeRef::ExprStringLiteral(string) => {
            build.text.0.clear();
            for part in &string.value {
                build.text.0.push_str(&part.value);
            }
            let value = word(build.words, NameText(&build.text.0));
            push_key_word(
                build.index,
                KeyWord::Literal(LiteralKind::String, Some(value)),
            )?;
            Some(None)
        }
        AnyNodeRef::ExprBytesLiteral(bytes) => {
            build.text.0.clear();
            for byte in bytes.value.bytes() {
                // Writing into a `String` does not fail.
                let _written: std::fmt::Result = write!(build.text.0, "{byte:02x}");
            }
            let value = word(build.words, NameText(&build.text.0));
            push_key_word(
                build.index,
                KeyWord::Literal(LiteralKind::Bytes, Some(value)),
            )?;
            Some(None)
        }
        AnyNodeRef::ExprNoneLiteral(_) => literal(build, LiteralKind::Null, None).map(|()| None),
        AnyNodeRef::ExprEllipsisLiteral(_) => {
            literal(build, LiteralKind::Ellipsis, None).map(|()| None)
        }
        AnyNodeRef::ExprTuple(tuple) => {
            push_key_word(build.index, KeyWord::Tuple)?;
            for element in &tuple.elts {
                let class = class_of(build.index, element)?;
                push_key_word(build.index, KeyWord::Child(class.representative))?;
            }
            push_key_word(build.index, KeyWord::EndTuple)?;
            Some(None)
        }
        AnyNodeRef::ExprUnaryOp(operation) => signed(build, operation),
        AnyNodeRef::ExprBinOp(operation) => folded(build, operation),
        _ => None,
    }
}

/// Finds the class of every constant of `file`, by node id: one pass from the last node to the first, so an element's class is
/// known before its tuple's is asked. A class is named by its last member in the file.
pub(crate) fn keyed(file: &SyntaxFile<'_>, build: &mut KeyBuild<'_>) {
    clear_keys(build.index);
    build.index.classes.resize(file.nodes.len(), None);
    for (id, node) in file.nodes.iter().enumerate().rev() {
        classify(build, Position(u32::try_from(id).unwrap_or(u32::MAX)), *node);
    }
}

/// Only equality inside a dictionary matters for its overwrites. Classify its key operands with the same evaluator.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) fn dictionary_keys<'a>(
    size: usize,
    dictionaries: impl Iterator<Item = &'a ruff_python_ast::ExprDict>,
    build: &mut KeyBuild<'_>,
) {
    fn operand(build: &mut KeyBuild<'_>, expr: &Expr) {
        let Some(at) = id_of(expr) else { return; };
        match expr {
            Expr::Tuple(tuple) => {
                for member in &tuple.elts { operand(build, member); }
            }
            Expr::UnaryOp(unary) => operand(build, &unary.operand),
            Expr::BinOp(binary) if matches!(binary.op, Operator::Add | Operator::Sub) => {
                operand(build, &binary.left);
                operand(build, &binary.right);
            }
            _ => {}
        }
        classify(build, at, expr.into());
    }
    clear_keys(build.index);
    build.index.classes.resize(size, None);
    for dict in dictionaries {
        if dict.items.iter().filter_map(|item| item.key.as_ref()).nth(1).is_some() {
            for key in dict.items.iter().filter_map(|item| item.key.as_ref()) {
                operand(build, key);
            }
        }
    }
}

fn classify(build: &mut KeyBuild<'_>, at: Position, node: AnyNodeRef<'_>) {
    build.index.scratch.clear();
    if let Some(number) = key_words(build, node) {
        let representative = intern_key(build.index, at);
        if let Some(slot) = build.index.classes.get_mut(usize::from(at)) {
            *slot = Some(KeyClass { representative, number });
        }
    }
}
