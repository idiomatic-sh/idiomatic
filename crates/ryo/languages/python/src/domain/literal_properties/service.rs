//! Classify literals from parser-owned scalar and decoded-token properties without evaluating them.

use crate::domain::{
    nodes::types::Node,
    syntax::{
        constants::{
            BOOLEAN_SCALAR, BYTES_SCALAR, COMPLEX_SCALAR, ELLIPSIS_SCALAR, FLOAT_SCALAR,
            INTEGER_SCALAR, NONE_SCALAR, SCALAR_IMAGINARY, SCALAR_KIND, SCALAR_VALUE,
        },
        service::{below, kind, literal, parent},
        types::{Kind, Literal},
    },
    tokens::types::Token,
};
use ryo::languages::{self, Position, Tree};

/// A known primitive category and the three explicitly limited predicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Properties {
    /// The closed primitive category.
    pub kind: &'static str,
    /// Whether a numeric parser value is exactly zero.
    pub zero: bool,
    /// Whether a decoded string or bytes parser value is empty.
    pub empty: bool,
    /// Whether the parsed boolean is `True`.
    pub boolean: bool,
}

/// A constant that is not a primitive occurrence, has a complete primitive payload, or lacks one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Classification {
    /// Not a primitive literal row, including synthetic interpolation fragments.
    NotPrimitive,
    /// A complete parser-derived result.
    Known(Properties),
    /// A primitive node exists but lacks required parser payload.
    MissingPayload,
}

/// Whether the parser's `Constant` node is a synthetic piece of an interpolated string.
fn interpolation_fragment(tree: &Tree<'_>, at: Position) -> bool {
    let mut child = at;
    let mut above = parent(tree, child);
    while above != child {
        match kind(tree, above) {
            // Values and format-spec expressions are real Python expressions. Constants
            // beneath them remain primitive occurrences even though they are in a string.
            Kind::Node(Node::FormattedValue | Node::Interpolation) => return false,
            Kind::Node(Node::JoinedStr | Node::TemplateStr) => return true,
            _ => {}
        }
        child = above;
        above = parent(tree, above);
    }
    false
}

/// Whether every decoded piece of this primitive string is empty.
fn decoded_string_empty(tree: &Tree<'_>, at: Position) -> Option<bool> {
    let mut found = false;
    let mut nonempty = false;
    for token in below(tree, at).filter(|each| kind(tree, *each) == Kind::Token(Token::String)) {
        let decoded = languages::decoded_at(tree.elements, token)?;
        found = true;
        nonempty |= !decoded.0.is_empty();
    }
    found.then_some(!nonempty)
}

fn payload<'a>(
    tree: &'a Tree<'_>,
    at: Position,
    property: languages::PropertyId,
) -> Option<&'a str> {
    languages::property_at(tree.elements, at, property).map(|text| text.0)
}

fn flag(kind: &'static str, zero: bool, empty: bool, boolean: bool) -> Classification {
    Classification::Known(Properties {
        kind,
        zero,
        empty,
        boolean,
    })
}

/// Classify one native or enabled embedded primitive occurrence. No spelling is parsed again.
#[must_use]
pub(crate) fn classify(tree: &Tree<'_>, at: Position) -> Classification {
    if kind(tree, at) != Kind::Node(Node::Constant) || interpolation_fragment(tree, at) {
        return Classification::NotPrimitive;
    }

    // Keep typed bytes separate from decoded text, even though both are constant expressions.
    let scalar_kind = payload(tree, at, SCALAR_KIND);
    if scalar_kind == Some(BYTES_SCALAR) {
        return payload(tree, at, SCALAR_VALUE).map_or(Classification::MissingPayload, |value| {
            flag("bytes", false, value.is_empty(), false)
        });
    }

    if literal(tree, at) == Literal::String {
        return decoded_string_empty(tree, at).map_or(Classification::MissingPayload, |empty| {
            flag("string", false, empty, false)
        });
    }

    let Some(scalar_kind) = scalar_kind else {
        return Classification::MissingPayload;
    };
    match scalar_kind {
        INTEGER_SCALAR => payload(tree, at, SCALAR_VALUE)
            .map_or(Classification::MissingPayload, |value| {
                flag("integer", value == "0", false, false)
            }),
        FLOAT_SCALAR => payload(tree, at, SCALAR_VALUE)
            .and_then(|bits| bits.parse::<u64>().ok())
            .map_or(Classification::MissingPayload, |bits| {
                flag("float", f64::from_bits(bits) == 0.0, false, false)
            }),
        COMPLEX_SCALAR => payload(tree, at, SCALAR_VALUE)
            .and_then(|real| real.parse::<u64>().ok())
            .zip(
                payload(tree, at, SCALAR_IMAGINARY)
                    .and_then(|imaginary| imaginary.parse::<u64>().ok()),
            )
            .map_or(Classification::MissingPayload, |(real, imaginary)| {
                flag(
                    "complex",
                    f64::from_bits(real) == 0.0 && f64::from_bits(imaginary) == 0.0,
                    false,
                    false,
                )
            }),
        BOOLEAN_SCALAR => payload(tree, at, SCALAR_VALUE)
            .and_then(|value| value.parse::<bool>().ok())
            .map_or(Classification::MissingPayload, |boolean| {
                flag("boolean", false, false, boolean)
            }),
        NONE_SCALAR => flag("null", false, false, false),
        ELLIPSIS_SCALAR => flag("ellipsis", false, false, false),
        _ => Classification::MissingPayload,
    }
}
