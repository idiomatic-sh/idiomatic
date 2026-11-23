//! Compare and render packed cells using the file's borrowed spelling dictionaries.
use super::{
    constants::MISSING_CELL,
    types::{CollationText, PackedScalar, ScalarSpelling},
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{CellKind, Flag};
use ryo_values::domain::table_words::types::PackedSlice;
use std::{borrow::Cow, cmp::Ordering};

/// A boolean result cell.
#[must_use]
pub const fn flag(value: Flag) -> PackedScalar {
    PackedScalar {
        kind: CellKind::Flag,
        word: CellWord(match value {
            Flag::Raised => 1,
            Flag::Lowered => 0,
        }),
    }
}

/// Only the true flag satisfies a predicate.
#[must_use]
#[inline]
pub fn truth(value: PackedScalar) -> Flag {
    if value.kind == CellKind::Flag && value.word == CellWord(1) {
        Flag::Raised
    } else {
        Flag::Lowered
    }
}

/// A symbol or file borrows its spelling. A number needs its decimal rendering.
pub fn spelling<'a>(
    value: PackedScalar,
    text: impl Fn(CellWord) -> PackedSlice<'a>,
    file: impl Fn(CellWord) -> PackedSlice<'a>,
) -> ScalarSpelling<'a> {
    ScalarSpelling(match value.kind {
        CellKind::Symbol => Cow::Borrowed(text(value.word).0),
        CellKind::File => Cow::Borrowed(file(value.word).0),
        CellKind::Number => Cow::Owned(
            i64::from_ne_bytes(value.word.0.to_ne_bytes())
                .to_string()
                .into_bytes(),
        ),
        CellKind::Flag => Cow::Borrowed(if truth(value) == Flag::Raised {
            b"true"
        } else {
            b"false"
        }),
        _ => Cow::Borrowed(&[]),
    })
}

/// Missing operands have no ordering. Cross-kind symbol comparisons use their spelling.
pub fn compare<'a>(
    left: PackedScalar,
    right: PackedScalar,
    text: impl Fn(CellWord) -> PackedSlice<'a>,
    file: impl Fn(CellWord) -> PackedSlice<'a>,
) -> Option<Ordering> {
    use CellKind::{File, Flag, Number, Symbol};
    if left.word == MISSING_CELL || right.word == MISSING_CELL {
        return None;
    }
    Some(match (left.kind, right.kind) {
        (Number, Number) => i64::from_ne_bytes(left.word.0.to_ne_bytes())
            .cmp(&i64::from_ne_bytes(right.word.0.to_ne_bytes())),
        (Symbol, Symbol) if left.word != right.word => text(left.word).0.cmp(text(right.word).0),
        (Number | Flag | File, Symbol) | (Symbol, Number | Flag | File) => {
            spelling(left, &text, &file)
                .0
                .cmp(&spelling(right, text, file).0)
        }
        _ => left.word.cmp(&right.word),
    })
}

/// Take a digit run or one non-whitespace character from the borrowed text.
fn unit<'a>(source: &mut CollationText<'a>) -> Option<CollationText<'a>> {
    let rest = source.0.trim_start_matches(char::is_whitespace);
    let first = rest.chars().next()?;
    let width = if first.is_ascii_digit() {
        rest.as_bytes()
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count()
    } else {
        first.len_utf8()
    };
    let (word, tail) = rest.split_at(width);
    source.0 = tail;
    Some(CollationText(word))
}

/// Order digit runs by magnitude; leading zeros make their spelling significant.
#[must_use]
pub fn natural_order(mut left: CollationText<'_>, mut right: CollationText<'_>) -> Ordering {
    loop {
        match (unit(&mut left), unit(&mut right)) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(l), Some(r)) => {
                let order = if l.0.starts_with(|c: char| c.is_ascii_digit())
                    && r.0.starts_with(|c: char| c.is_ascii_digit())
                    && !l.0.starts_with('0')
                    && !r.0.starts_with('0')
                {
                    l.0.len().cmp(&r.0.len()).then_with(|| l.0.cmp(r.0))
                } else {
                    l.0.cmp(r.0)
                };
                if !order.is_eq() {
                    return order;
                }
            }
        }
    }
}
