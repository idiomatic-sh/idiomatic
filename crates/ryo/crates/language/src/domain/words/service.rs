//! The `words` service: free functions named by verb over the relations' types, with no IO and no
//! ports. A value is written as a tag and the words it holds, and read back from them.

use crate::domain::relations::types::{
    Bits, CellKind, FileId, Flag, Half, Integer, Name, NamePlace, NodeId, Offset, Origin,
    PlaceNumber, Span, TextHash, Value, Words, WorkerNumber,
};
use crate::domain::terms::types::Symbol;

/// The words a kind is written as.
fn kind_word(kind: CellKind) -> Bits {
    Bits(match kind {
        CellKind::Node => 0,
        CellKind::Symbol => 1,
        CellKind::File => 2,
        CellKind::Span => 3,
        CellKind::Number => 4,
        CellKind::Flag => 5,
    })
}

/// The kind a word stands for.
fn kind_from_word(word: Bits) -> Option<CellKind> {
    [
        CellKind::Node,
        CellKind::Symbol,
        CellKind::File,
        CellKind::Span,
        CellKind::Number,
        CellKind::Flag,
    ]
    .into_iter()
    .find(|kind| kind_word(*kind) == word)
}

/// Writes a kind as one word.
pub fn encode_kind(kind: CellKind, out: &mut Words) {
    out.0.push(kind_word(kind).0);
}

/// Reads a kind from the next word.
pub fn decode_kind(words: &mut dyn Iterator<Item = Bits>) -> Option<CellKind> {
    kind_from_word(words.next()?)
}

/// Writes a value as words, so it can leave a file as numbers: a tag, then what the value holds.
/// A name that left its file takes two words after the tag: its hash and the number of its place.
pub fn encode_value(value: Value, out: &mut Words) {
    match value {
        Value::Node(node) => out.0.extend([0, node.0]),
        Value::Symbol(symbol) => out.0.extend([1, u64::from(symbol.0)]),
        Value::Name(name) => out.0.extend([2, name.hash.0, u64::from(name.place.0)]),
        Value::File(file) => out.0.extend([3, u64::from(file.0)]),
        Value::Span(span) => {
            out.0.extend([
                4,
                (u64::from(span.start.0) << 32_i32) | u64::from(span.end.0),
            ]);
        }
        Value::Number(number) => out.0.extend([5, number.0.cast_unsigned()]),
        Value::Flag(flag) => out.0.extend([6, u64::from(flag == Flag::Raised)]),
    }
}

/// The low half of a word.
fn low(word: Bits) -> Half {
    Half(u32::try_from(word.0 & 0xffff_ffff).unwrap_or(u32::MAX))
}

/// The high half of a word.
fn high(word: Bits) -> Half {
    Half(u32::try_from(word.0 >> 32).unwrap_or(u32::MAX))
}

/// Reads a value from the next words.
pub fn decode_value(words: &mut dyn Iterator<Item = Bits>) -> Option<Value> {
    let mut next = || words.next();
    Some(match next()?.0 {
        0 => Value::Node(NodeId(next()?.0)),
        1 => Value::Symbol(Symbol(low(next()?).0)),
        2 => {
            let hash = TextHash(next()?.0);
            let place = PlaceNumber(low(next()?).0);
            Value::Name(Name { hash, place })
        }
        3 => Value::File(FileId(low(next()?).0)),
        4 => {
            let span = next()?;
            Value::Span(Span {
                start: Offset(high(span).0),
                end: Offset(low(span).0),
            })
        }
        5 => Value::Number(Integer(next()?.0.cast_signed())),
        6 => Value::Flag(if next()?.0 == 0 {
            Flag::Lowered
        } else {
            Flag::Raised
        }),
        _ => return None,
    })
}

/// Writes the places of a table's names as words: the number of them, then for each its file and origin, and where it starts and ends.
pub fn encode_places(places: &[NamePlace], out: &mut Words) {
    out.0.push(u64::try_from(places.len()).unwrap_or(0));
    for place in places {
        let origin = match place.origin {
            Origin::Source => 0,
            Origin::Derived(worker) => u64::from(worker.0) + 1,
        };
        out.0.extend([
            (u64::from(place.file.0) << 32_i32) | origin,
            (u64::from(place.start.0) << 32_i32) | u64::from(place.end.0),
        ]);
    }
}

/// Reads the places written by [`encode_places`] onto `into`.
pub fn decode_places(
    words: &mut dyn Iterator<Item = Bits>,
    into: &mut Vec<NamePlace>,
) -> Option<()> {
    let count = words.next()?.0;
    for _ in 0..count {
        let place = words.next()?;
        let span = words.next()?;
        let origin = match low(place).0 {
            0 => Origin::Source,
            worker => Origin::Derived(WorkerNumber(worker - 1)),
        };
        into.push(NamePlace {
            origin,
            file: FileId(high(place).0),
            start: Offset(high(span).0),
            end: Offset(low(span).0),
        });
    }
    Some(())
}
