//! The `table_texts` service: how a value reads as text or as a number when it is compared, with no IO and no ports.

use ryo_language::domain::names::service::{derived_of, places_of};
use ryo_language::domain::symbols::service::{file_in, place_of, text_of};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{
    CROSS_FILE, FileId, Flag, Integer, Name, Origin, Value,
};
use ryo_relations::domain::terms::types::Symbol;

use crate::domain::table_texts::types::{Datum, TextView, Texts};
use ryo_values::domain::plan_parts::types::{FlagState, WholeNumber};

/// The text of a name, or nothing when the texts do not hold it.
#[must_use]
pub fn symbol_text<'a>(texts: &Texts<'a>, symbol: Symbol) -> Option<TextView<'a>> {
    text_of(texts.symbols, ByteSlice(texts.source.0), symbol).map(|found| TextView(found.0))
}

/// The text of a name that has left its file, when its worker's arena holds it.
#[must_use]
pub fn name_text<'a>(texts: &Texts<'a>, name: Name) -> Option<TextView<'a>> {
    let place = place_of(name, places_of(texts.symbols), texts.symbols)?;
    let Origin::Derived(worker) = place.origin else {
        return None;
    };
    let arena = if worker == CROSS_FILE {
        &derived_of(texts.symbols).0
    } else {
        &texts
            .arenas
            .get(usize::try_from(worker.0).unwrap_or(usize::MAX))?
            .0
    };
    let text = arena.get(
        usize::try_from(place.start.0).unwrap_or(usize::MAX)
            ..usize::try_from(place.end.0).unwrap_or(usize::MAX),
    )?;
    match std::str::from_utf8(text) {
        Ok(text) => Some(TextView(text)),
        // bytes of a name that are not text have no text to compare, which the caller reads as no text
        Err(_) => None,
    }
}

/// The path of a file, or nothing when the texts do not hold it.
#[must_use]
pub(crate) fn file_text<'a>(texts: &Texts<'a>, file: FileId) -> Option<TextView<'a>> {
    file_in(texts.paths, file).map(|path| TextView(&path.0))
}

/// What a value reads as when it is compared.
#[must_use]
pub fn datum<'a>(value: Value, texts: &Texts<'a>) -> Datum<'a> {
    match value {
        Value::Node(node) => Datum::Number(WholeNumber(node.0.cast_signed())),
        Value::Symbol(symbol) => Datum::Text(symbol_text(texts, symbol).unwrap_or_default()),
        Value::Name(name) => name_text(texts, name).map_or(
            Datum::Number(WholeNumber(name.hash.0.cast_signed())),
            Datum::Text,
        ),
        Value::File(file) => Datum::Text(file_text(texts, file).unwrap_or_default()),
        Value::Span(span) => {
            if ryo_relations::domain::relations::service::location(span)
                == ryo_relations::domain::relations::types::Locatedness::Located
            {
                Datum::Number(WholeNumber(i64::from(span.start.0)))
            } else {
                Datum::Unlocated
            }
        }
        Value::Number(Integer(number)) => Datum::Number(WholeNumber(number)),
        Value::Flag(flag) => Datum::Flag(if flag == Flag::Raised {
            FlagState::Raised
        } else {
            FlagState::Lowered
        }),
    }
}

/// Where a text no symbol of the file holds yet is numbered: after every symbol, as interning would number it.
const FRESH: Symbol = Symbol(1 << 31);

/// The symbol main's engine interned a text cell to, or any other cell as it is.
pub(crate) fn canonical_cell(
    value: Value,
    texts: &Texts<'_>,
    fresh: &mut Vec<Value>,
) -> Result<Value, crate::domain::contract::types::PortableOperatorFault> {
    let text = match value {
        Value::Symbol(symbol) => match symbol_text(texts, symbol) {
            Some(text) => text,
            None => return Ok(value),
        },
        Value::Name(name) => name_text(texts, name)
            .ok_or(crate::domain::contract::types::PortableOperatorFault::ShapeMismatch)?,
        Value::Node(_) | Value::File(_) | Value::Span(_) | Value::Number(_) | Value::Flag(_) => {
            return Ok(value);
        }
    };
    let bytes = ByteSlice(text.0.as_bytes());
    if let Some(symbol) = ryo_language::domain::symbols::service::find_symbol(
        texts.symbols,
        ByteSlice(texts.source.0),
        bytes,
    ) {
        return Ok(Value::Symbol(symbol));
    }
    let place = match fresh
        .iter()
        .position(|held| text_of_cell(*held, texts).is_some_and(|held| held.0 == text.0))
    {
        Some(place) => place,
        None => {
            fresh.push(value);
            fresh.len() - 1
        }
    };
    Ok(Value::Symbol(Symbol(
        FRESH.0 + u32::try_from(place).unwrap_or(u32::MAX >> 1),
    )))
}

/// The text of a symbol or name cell.
pub(crate) fn text_of_cell<'t>(value: Value, texts: &Texts<'t>) -> Option<TextView<'t>> {
    match value {
        Value::Symbol(symbol) => symbol_text(texts, symbol),
        Value::Name(name) => name_text(texts, name),
        Value::Node(_) | Value::File(_) | Value::Span(_) | Value::Number(_) | Value::Flag(_) => {
            None
        }
    }
}
