//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the element table holds and back, says its name, and reads the kind of an element of a file's
//! syntax tree: a position is an element of the table; its kind is the language's own.

use crate::domain::syntax::reading::{Interner, Syntax};
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
pub(crate) fn kind(tree: &impl Syntax, at: languages::Position) -> Kind {
    kind_of(tree.kind_bits(at))
}

/// Whether the element at `at` is a token, a leaf, and not a node.
#[must_use]
pub(crate) fn is_token(tree: &impl Syntax, at: languages::Position) -> Leaf {
    if !tree.is_leaf(at) {
        Leaf::Node
    } else {
        Leaf::Token
    }
}

/// The parent of the element at `at`; the root is its own.
#[must_use]
pub(crate) fn parent(tree: &impl Syntax, at: languages::Position) -> languages::Position {
    tree.parent_of(at)
}

/// The name of the kind of the element at `at`, as the `kind` column of the tree holds it.
pub(crate) fn kind_symbol(
    tree: &impl Syntax,
    symbols: &mut impl Interner,
    at: languages::Position,
) -> languages::Symbol {
    let element = kind(tree, at);
    symbols.vocabulary(tree.bytes(), kind_bits(element), kind_name(element))
}

/// The last element below the element at `at`, in pre-order; `at` itself when it has none.
#[must_use]
pub(crate) fn last(tree: &impl Syntax, at: languages::Position) -> languages::Position {
    tree.last_of(at)
}

/// The bytes of the element at `at`, as the file spells them.
pub(crate) fn written<'a>(tree: &'a impl Syntax, at: languages::Position) -> languages::NameText<'a> {
    let from = usize::try_from(tree.start_of(at).0).unwrap_or(usize::MAX);
    let to = usize::try_from(tree.end_of(at).0).unwrap_or(usize::MAX);
    languages::NameText(
        tree.bytes()
            .0
            .get(from..to)
            // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
            .map(|bytes| std::str::from_utf8(bytes).unwrap_or_default())
            .unwrap_or_default(),
    )
}

/// Whether a comment written as `raw` is documentation.
#[must_use]
pub(crate) fn documentation_of(raw: &str) -> languages::Flag {
    if raw.starts_with(DOC_OPEN) && raw != EMPTY_BLOCK {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// Whether a numeric literal written as `raw`, of the parser's `kind` and `value`, writes digits its value gives back.
#[must_use]
pub(crate) fn exactness_of(
    kind: Option<languages::NameText<'_>>,
    value: Option<languages::NameText<'_>>,
    raw: &str,
) -> languages::Flag {
    if kind.map(|kind| kind.0) != Some(FLOAT_NUMBER) {
        return languages::Flag::Raised;
    }
    let Some(value) = value.and_then(|value| match value.0.parse::<f64>() {
        Ok(parsed) => Some(parsed),
        Err(_) => None,
    }) else {
        // a Number without its decoded value has no digits to give back
        return languages::Flag::Lowered;
    };
    let raw = raw.as_bytes();
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
pub(crate) fn parameter_kind(tree: &impl Syntax, at: languages::Position) -> languages::ParameterKind {
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
    tree: &impl Syntax,
    at: languages::Position,
    property: languages::PropertyId,
) -> Option<languages::Offset> {
    let Ok(offset) = tree.property_of(at, property)?
        .0
        .parse()
    else {
        return None; // the adapter writes a decimal offset: text that is none is no offset
    };
    Some(languages::Offset(offset))
}
