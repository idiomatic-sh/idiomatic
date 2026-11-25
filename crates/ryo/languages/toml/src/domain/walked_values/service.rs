//! The `walked_values` service: free functions over a walked file's syntax that say what a key or a value is as a row's cell: the
//! text it is written as, what the library decoded of it, and the whole number, yes or no and base it stands for.

use crate::domain::file_view::service::{decoded, kind, parent, word, written};
use crate::domain::file_view::types::Syntax;
use crate::domain::model::constants::{DECIMAL, RADIX_PREFIXES};
use crate::domain::model::types::Stringness;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::value_kind;
use crate::domain::syntax::types::Kind;
use crate::domain::walked_documents::service::key_text;
use ryo::languages::{self, FileWords, Position, Symbol, ValueKind};

/// The key the part at `at` says, as a name: a bare key is itself, a quoted one is what the library decoded.
pub(crate) fn key_name(syntax: &Syntax<'_>, words: &mut FileWords, at: Position) -> Symbol {
    word(words, key_text(syntax, at).0)
}

/// The text the element at `at` spans, as a name.
pub(crate) fn spanned(syntax: &Syntax<'_>, words: &mut FileWords, at: Position) -> Symbol {
    word(words, written(syntax, at))
}

/// What the token at `at` says as the library decoded it, as a name; nothing for a token it did not decode, or refused.
pub(crate) fn decoded_name(syntax: &Syntax<'_>, words: &mut FileWords, at: Position) -> Option<Symbol> {
    decoded(syntax, at).map(|text| word(words, text))
}

/// What the string value at `at` says, as the library decoded it; no name for a value that is no string, or one it refused.
pub(crate) fn said(syntax: &Syntax<'_>, words: &mut FileWords, at: Option<Position>) -> Symbol {
    at.filter(|each| value_kind(kind(syntax, *each)) == Some(ValueKind::String))
        .and_then(|each| decoded_name(syntax, words, each))
        .unwrap_or(languages::NO_SYMBOL)
}

/// The value at `at` as a yes or no when it is a boolean the library decoded: raised for `true`; lowered for every other.
#[must_use]
pub(crate) fn boolean(syntax: &Syntax<'_>, at: Option<Position>) -> languages::Flag {
    let raised = at
        .filter(|each| value_kind(kind(syntax, *each)) == Some(ValueKind::Boolean))
        .and_then(|each| decoded(syntax, each))
        // a decoded value that does not read as the kind its node says is no value, which the column holds as its default
        .map(|text| text.parse::<bool>().unwrap_or_default())
        .unwrap_or_default();
    if raised {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// The value at `at` as a whole number when it is an integer the library decoded; zero for every other kind and for no value.
#[must_use]
pub(crate) fn integer(syntax: &Syntax<'_>, at: Option<Position>) -> languages::Integer {
    languages::Integer(
        at.filter(|each| value_kind(kind(syntax, *each)) == Some(ValueKind::Integer))
            .and_then(|each| decoded(syntax, each))
            // a decoded value that does not read as the kind its node says is no value, which the column holds as its default
            .map(|text| text.parse::<i64>().unwrap_or_default())
            .unwrap_or_default(),
    )
}

/// The value at `at` as the library decoded it when it is of `wanted` kind, a float or a date-time, as text; none for every other kind
/// and for no value.
pub(crate) fn decoded_as(
    syntax: &Syntax<'_>,
    words: &mut FileWords,
    at: Option<Position>,
    wanted: ValueKind,
) -> Symbol {
    at.filter(|each| value_kind(kind(syntax, *each)) == Some(wanted))
        .and_then(|each| decoded_name(syntax, words, each))
        .unwrap_or(languages::NO_SYMBOL)
}

/// The base the integer at `at` is written in, by its prefix as the TOML grammar writes it; zero for every other kind and for no value.
#[must_use]
pub(crate) fn radix(syntax: &Syntax<'_>, at: Option<Position>) -> languages::Integer {
    languages::Integer(
        at.filter(|each| value_kind(kind(syntax, *each)) == Some(ValueKind::Integer))
            .map_or(0, |each| {
                let text = written(syntax, each);
                RADIX_PREFIXES
                    .iter()
                    .find(|(prefix, _)| text.starts_with(prefix))
                    .map_or(DECIMAL, |(_, base)| *base)
            }),
    )
}

/// Whether the string at `at` is a value or a part of a key.
#[must_use]
pub(crate) fn stringness(syntax: &Syntax<'_>, at: Position) -> Stringness {
    if kind(syntax, parent(syntax, at)) != Kind::Node(Node::Key)
        && value_kind(kind(syntax, at)) == Some(ValueKind::String)
    {
        Stringness::Value
    } else {
        Stringness::Other
    }
}
