//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the element table holds and back, says its name, and reads the kind of an element of a file's
//! syntax tree: a position is an element of the table; its kind is the language's own.

use ryo::languages::{self, Tree};

use super::constants::{
    DOC_OPEN, EMPTY_BLOCK, FLOAT_NUMBER, LEGACY_OCTAL_PREFIX, NUMERIC_KIND, NUMERIC_VALUE, RECEIVER,
};
use super::types::{Kind, Leaf, PatternNode};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

ryo::languages::kinds!(pub Kind { Node(Node), Token(Token), Extra(Extra), Pattern(PatternNode) } unknown Kind::Extra(Extra::Error));

/// The kind of the element at `at`.
#[must_use]
pub(crate) fn kind(tree: &Tree<'_>, at: languages::Position) -> Kind {
    kind_of(languages::kind_at(tree.elements, at))
}

/// Whether the element at `at` is a token, a leaf, and not a node.
#[must_use]
pub(crate) fn is_token(tree: &Tree<'_>, at: languages::Position) -> Leaf {
    if languages::flags_at(tree.elements, at).0 & languages::TOKEN.0 == 0 {
        Leaf::Node
    } else {
        Leaf::Token
    }
}

/// The parent of the element at `at`; the root is its own.
#[must_use]
pub(crate) fn parent(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    languages::parent_at(tree.elements, at)
}

/// The name of the kind of the element at `at`, as the `kind` column of the tree holds it.
pub(crate) fn kind_symbol(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let element = kind(tree, at);
    languages::vocabulary_symbol(symbols, tree.source, kind_bits(element), kind_name(element))
}

/// The text of the token at `at`; no text for a node.
#[must_use]
pub(crate) fn token_text(tree: &Tree<'_>, at: languages::Position) -> languages::Symbol {
    if is_token(tree, at) == Leaf::Token {
        languages::text_at(tree.elements, at)
    } else {
        languages::NO_SYMBOL
    }
}

/// The last element below the element at `at`, in pre-order; `at` itself when it has none.
#[must_use]
pub(crate) fn last(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    languages::last_at(tree.elements, at)
}

/// The bytes of the element at `at`, as the file spells them.
pub(crate) fn written<'a>(tree: &Tree<'a>, at: languages::Position) -> languages::NameText<'a> {
    let from = usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX);
    let to = usize::try_from(languages::end_at(tree.elements, at).0).unwrap_or(usize::MAX);
    languages::NameText(
        tree.source
            .0
            .get(from..to)
            // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
            .map(|bytes| std::str::from_utf8(bytes).unwrap_or_default())
            .unwrap_or_default(),
    )
}

/// Whether the comment at `at` is documentation, as TypeScript reads it: a block comment that opens with `/**`, the empty `/**/`
/// aside.
#[must_use]
pub(crate) fn documentation(tree: &Tree<'_>, at: languages::Position) -> languages::Flag {
    let raw = written(tree, at).0;
    if raw.starts_with(DOC_OPEN) && raw != EMPTY_BLOCK {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// Whether the numeric literal at `at` writes digits its value gives back, the `exact-digits` cell: a BigInt is exact at any
/// size. A Number's digits are its spelling without `_` and leading zeros, copied to the stack: a `0x`, `0o` or `0b` prefix in
/// lower case and the digits after it, a legacy octal `017` as `0o17`, or else the decimal digits before the exponent.
#[must_use]
pub(crate) fn exactness(tree: &Tree<'_>, at: languages::Position) -> languages::Flag {
    if languages::property_at(tree.elements, at, NUMERIC_KIND).map(|kind| kind.0)
        != Some(FLOAT_NUMBER)
    {
        return languages::Flag::Raised;
    }
    let Some(value) = languages::property_at(tree.elements, at, NUMERIC_VALUE).and_then(|value| {
        match value.0.parse::<f64>() {
            Ok(parsed) => Some(parsed),
            Err(_) => None,
        }
    }) else {
        // a Number without its decoded value has no digits to give back
        return languages::Flag::Lowered;
    };
    let raw = written(tree, at).0.as_bytes();
    let (prefix, rest) = match raw {
        [
            b'0',
            prefix @ (b'x' | b'X' | b'o' | b'O' | b'b' | b'B'),
            rest @ ..,
        ] => (Some([b'0', prefix.to_ascii_lowercase()]), rest),
        [b'0', rest @ ..]
            if !rest.is_empty() && rest.iter().all(|byte| matches!(byte, b'0'..=b'7')) =>
        {
            (Some(LEGACY_OCTAL_PREFIX), rest)
        }
        decimal => (None, decimal),
    };
    let decimal = prefix.is_none();
    let mut digits = [0_u8; languages::WRITTEN_DIGITS_BYTES];
    let mut length = 0;
    let significant = rest
        .iter()
        .take_while(|byte| !decimal || !matches!(byte, b'e' | b'E'))
        .filter(|byte| **byte != b'_')
        .skip_while(|byte| **byte == b'0');
    for byte in prefix.iter().flatten().chain(significant) {
        let Some(slot) = digits.get_mut(length) else {
            // more digits than binary64's widest whole value has bits: never exact
            return languages::Flag::Lowered;
        };
        *slot = *byte;
        length += 1;
    }
    match digits.get(..length).map(std::str::from_utf8) {
        Some(Ok(text)) => languages::exact_digits(
            languages::NameText(text),
            languages::FloatBits(value.to_bits()),
        ),
        // digits copied from a numeric literal are ASCII; a cut that is not text writes no digits to give back
        Some(Err(_)) | None => languages::Flag::Lowered,
    }
}

/// How the parameter at `at` takes its argument: a rest element takes the arguments left over, a `this` parameter types the
/// receiver, and every other is positional.
#[must_use]
pub(crate) fn parameter_kind(tree: &Tree<'_>, at: languages::Position) -> languages::ParameterKind {
    match kind(tree, at) {
        Kind::Node(Node::RestElement) => languages::ParameterKind::VariadicPositional,
        Kind::Node(Node::Identifier)
            if written(tree, at)
                .0
                .strip_prefix(RECEIVER)
                .is_some_and(|rest| {
                    rest.chars().next().is_none_or(|character| {
                        !(character == '_' || character == '$' || character.is_alphanumeric())
                    })
                }) =>
        {
            languages::ParameterKind::Receiver
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) | Kind::Pattern(_) => {
            languages::ParameterKind::Positional
        }
    }
}

/// The offset a property of the element at `at` holds, in decimal, when it holds one.
#[must_use]
pub(crate) fn offset(
    tree: &Tree<'_>,
    at: languages::Position,
    property: languages::PropertyId,
) -> Option<languages::Offset> {
    let Ok(offset) = languages::property_at(tree.elements, at, property)?
        .0
        .parse()
    else {
        return None; // the adapter writes a decimal offset: text that is none is no offset
    };
    Some(languages::Offset(offset))
}
