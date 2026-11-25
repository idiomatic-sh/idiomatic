//! The `walked_values` service: free functions over a walked file's syntax that say what a member or a value is as a row's cell: the
//! place it holds in its container, the text it is written as, what the library decoded of it, and the number, yes or no and kind it
//! stands for.

use crate::domain::extras::types::Extra;
use crate::domain::file_view::service::{children, decoded, kind, parent, word, written};
use crate::domain::file_view::types::Syntax;
use crate::domain::model::types::{Counted, Model};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{
    BLOCK_COMMENT_CLOSE, BLOCK_COMMENT_OPEN, DECIMAL, LINE_COMMENT_OPEN,
};
use crate::domain::syntax::service::is_value;
use crate::domain::syntax::types::{Keyness, Kind, Valueness};
use crate::domain::tokens::types::Token;
use ryo::languages::{self, FileWords, MappingKind, Position, Symbol, ValueKind, node_table};

/// What a value is: nothing for an element that is no value.
#[must_use]
pub(crate) fn value_kind(syntax: &Syntax<'_>, at: Position) -> Option<ValueKind> {
    match kind(syntax, at) {
        Kind::Token(Token::String) => Some(ValueKind::String),
        Kind::Token(Token::Number) => Some(if written(syntax, at).contains(['.', 'e', 'E']) {
            ValueKind::Float
        } else {
            ValueKind::Integer
        }),
        Kind::Token(Token::True | Token::False) => Some(ValueKind::Boolean),
        Kind::Token(Token::Null) => Some(ValueKind::Null),
        Kind::Node(Node::Array) => Some(ValueKind::Sequence),
        Kind::Node(Node::Object) => Some(ValueKind::InlineMapping),
        Kind::Node(Node::JsonText | Node::Member)
        | Kind::Token(
            Token::BeginArray
            | Token::BeginObject
            | Token::EndArray
            | Token::EndObject
            | Token::NameSeparator
            | Token::ValueSeparator
            | Token::Ws,
        )
        | Kind::Extra(Extra::Error | Extra::LineComment | Extra::BlockComment) => None,
    }
}

/// Whether the string `at` is the name of a member: the first string of the member it is in.
#[must_use]
pub(crate) fn keyness(syntax: &Syntax<'_>, at: Position) -> Keyness {
    let member = parent(syntax, at);
    if kind(syntax, member) == Kind::Node(Node::Member)
        && child_of_kind(syntax, member, Kind::Token(Token::String)) == Some(at)
    {
        Keyness::Key
    } else {
        Keyness::Other
    }
}

/// The first child of `at` of `wanted` kind.
fn child_of_kind(syntax: &Syntax<'_>, at: Position, wanted: Kind) -> Option<Position> {
    children(syntax, at).find(|child| kind(syntax, *child) == wanted)
}

/// The value of the member: the last of its children that is a value.
#[must_use]
pub(crate) fn value_of(syntax: &Syntax<'_>, member: Position) -> Option<Position> {
    children(syntax, member)
        .filter(|child| is_value(kind(syntax, *child)) == Valueness::Value)
        .last()
}

/// The element that holds the container `container`: the file's value, the name of the member whose value it is, or itself.
#[must_use]
pub(crate) fn holder(syntax: &Syntax<'_>, container: Position) -> Position {
    let above = parent(syntax, container);
    match kind(syntax, above) {
        Kind::Node(Node::JsonText) => above,
        Kind::Node(Node::Member) => {
            child_of_kind(syntax, above, Kind::Token(Token::String)).unwrap_or(container)
        }
        Kind::Node(Node::Object | Node::Array) | Kind::Token(_) | Kind::Extra(_) => container,
    }
}

/// The element a mapping is known by: the file when the object is the file's value, else the object.
#[must_use]
pub(crate) fn mapping_id(syntax: &Syntax<'_>, object: Position) -> Position {
    let above = parent(syntax, object);
    if kind(syntax, above) == Kind::Node(Node::JsonText) {
        above
    } else {
        object
    }
}

/// How the object is written.
#[must_use]
pub(crate) fn mapping_kind(syntax: &Syntax<'_>, object: Position) -> MappingKind {
    match kind(syntax, parent(syntax, object)) {
        Kind::Node(Node::JsonText) => MappingKind::Standard,
        Kind::Node(Node::Array) => MappingKind::SequenceMember,
        Kind::Node(Node::Object | Node::Member) | Kind::Token(_) | Kind::Extra(_) => {
            MappingKind::Inline
        }
    }
}

/// The text the element at `at` spans, as a name.
pub(crate) fn spanned(syntax: &Syntax<'_>, words: &mut FileWords, at: Position) -> Symbol {
    word(words, written(syntax, at))
}

/// What the library decoded of the string at `at`, as a name; the text it is written as when it decoded none.
pub(crate) fn decoded_name(syntax: &Syntax<'_>, words: &mut FileWords, at: Position) -> Symbol {
    match decoded(syntax, at) {
        Some(text) => word(words, text),
        None => spanned(syntax, words, at),
    }
}

/// What the string value at `at` says, as the library decoded it; no name for any other value.
pub(crate) fn said(syntax: &Syntax<'_>, words: &mut FileWords, at: Position) -> Symbol {
    if kind(syntax, at) == Kind::Token(Token::String) {
        decoded(syntax, at).map_or(languages::NO_SYMBOL, |text| word(words, text))
    } else {
        languages::NO_SYMBOL
    }
}

/// The value at `at` as a yes or no: raised for `true`.
#[must_use]
pub(crate) fn boolean(syntax: &Syntax<'_>, at: Position) -> languages::Flag {
    if kind(syntax, at) == Kind::Token(Token::True) {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// The value at `at` as a whole number when it is one; zero for every other.
#[must_use]
pub(crate) fn integer(syntax: &Syntax<'_>, at: Position) -> languages::Integer {
    languages::Integer(if value_kind(syntax, at) == Some(ValueKind::Integer) {
        written(syntax, at).parse::<i64>().unwrap_or_default()
    } else {
        0
    })
}

/// The value at `at` as a float in the shortest decimal form that reads back as it; none for every other.
pub(crate) fn float(syntax: &Syntax<'_>, words: &mut FileWords, at: Position) -> Symbol {
    if value_kind(syntax, at) != Some(ValueKind::Float) {
        return languages::NO_SYMBOL;
    }
    let Ok(number) = written(syntax, at).parse::<f64>() else {
        return languages::NO_SYMBOL;
    };
    // `Debug` writes an `f64` in the shortest decimal form that reads back as it, an exponent where one is shorter; `Display` never
    // writes one.
    #[expect(
        clippy::use_debug,
        reason = "`Debug` is the shortest round-trip form of an `f64`, which the column is defined to hold"
    )]
    let shown = format!("{number:?}");
    word(words, &shown)
}

/// The base the integer at `at` is written in; zero for every other.
#[must_use]
pub(crate) fn radix(syntax: &Syntax<'_>, at: Position) -> languages::Integer {
    languages::Integer(if value_kind(syntax, at) == Some(ValueKind::Integer) {
        DECIMAL
    } else {
        0
    })
}

/// The text of the comment at `at` between its delimiters, as a name.
pub(crate) fn comment_text(syntax: &Syntax<'_>, words: &mut FileWords, at: Position) -> Symbol {
    let text = written(syntax, at);
    let inside = match kind(syntax, at) {
        Kind::Extra(Extra::BlockComment) => text
            .strip_prefix(BLOCK_COMMENT_OPEN)
            .map(|rest| rest.strip_suffix(BLOCK_COMMENT_CLOSE).unwrap_or(rest)),
        Kind::Extra(Extra::LineComment) => text.strip_prefix(LINE_COMMENT_OPEN),
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(Extra::Error) => None,
    };
    word(words, inside.unwrap_or_default())
}

/// The place each member of an object and each value of an array holds in its container, by the element, worked out once for a file.
pub(crate) fn places(syntax: &Syntax<'_>, model: &mut Model) {
    let count = usize::try_from(node_table::element_count(syntax.table).0).unwrap_or(usize::MAX);
    model.places.clear();
    model.places.resize(count, languages::Integer(0));
    for id in 0..u32::try_from(count).unwrap_or(u32::MAX) {
        let container = Position(id);
        let counting = match kind(syntax, container) {
            Kind::Node(Node::Object) => Counted::Members,
            Kind::Node(Node::Array) => Counted::Values,
            Kind::Node(Node::JsonText | Node::Member) | Kind::Token(_) | Kind::Extra(_) => continue,
        };
        let mut next = languages::Integer(0);
        for child in children(syntax, container) {
            let counts = match counting {
                Counted::Members => kind(syntax, child) == Kind::Node(Node::Member),
                Counted::Values => is_value(kind(syntax, child)) == Valueness::Value,
            };
            if counts
                && let Some(slot) = model
                    .places
                    .get_mut(usize::try_from(child.0).unwrap_or(usize::MAX))
            {
                *slot = next;
                next = languages::Integer(next.0 + 1);
            }
        }
    }
}

/// The place the member or value at `at` holds in its container.
#[must_use]
pub(crate) fn place(model: &Model, at: Position) -> languages::Integer {
    model
        .places
        .get(usize::try_from(at.0).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or_default()
}

/// The place an object holds among the mappings of an array, otherwise zero.
#[must_use]
pub(crate) fn mapping_ordinal(
    syntax: &Syntax<'_>,
    model: &Model,
    object: Position,
) -> languages::Integer {
    if kind(syntax, parent(syntax, object)) == Kind::Node(Node::Array) {
        place(model, object)
    } else {
        languages::Integer(0)
    }
}
