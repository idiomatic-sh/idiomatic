//! Classify literals from parser-owned scalar and decoded-token properties without evaluating them.

use super::constants;
use super::types::{Interpolated, LiteralClassification, Properties};
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
use ryo::languages::{self, Flag, NameText, Position, Tree};

/// Whether the parser's `Constant` node is a synthetic piece of an interpolated string.
fn interpolation_fragment(tree: &Tree<'_>, at: Position) -> Interpolated {
    let mut child = at;
    let mut above = parent(tree, child);
    while above != child {
        match kind(tree, above) {
            // Values and format-spec expressions are real Python expressions. Constants
            // beneath them remain primitive occurrences even though they are in a string.
            Kind::Node(Node::FormattedValue | Node::Interpolation) => return Interpolated(false),
            Kind::Node(Node::JoinedStr | Node::TemplateStr) => return Interpolated(true),
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
        }
        child = above;
        above = parent(tree, above);
    }
    Interpolated(false)
}

/// Whether every decoded piece of this primitive string is empty.
fn decoded_string_empty(tree: &Tree<'_>, at: Position) -> Option<Flag> {
    let mut found = false;
    let mut nonempty = false;
    for token in below(tree, at).filter(|each| kind(tree, *each) == Kind::Token(Token::String)) {
        let decoded = languages::decoded_at(tree.elements, token)?;
        found = true;
        nonempty |= !decoded.0.is_empty();
    }
    found.then_some(if nonempty {
        Flag::Lowered
    } else {
        Flag::Raised
    })
}

/// Read one parser-owned scalar payload without interpreting source spelling.
fn payload<'a>(
    tree: &'a Tree<'_>,
    at: Position,
    property: languages::PropertyId,
) -> Option<NameText<'a>> {
    languages::property_at(tree.elements, at, property)
}

/// Build a complete primitive classification from its parsed category and predicates.
fn flag(kind: NameText<'static>, zero: Flag, empty: Flag, boolean: Flag) -> LiteralClassification {
    LiteralClassification::Known(Properties {
        kind,
        zero,
        empty,
        boolean,
    })
}

/// Classify one native or enabled embedded primitive occurrence. No spelling is parsed again.
#[must_use]
pub(crate) fn classify(tree: &Tree<'_>, at: Position) -> LiteralClassification {
    if kind(tree, at) != Kind::Node(Node::Constant) || interpolation_fragment(tree, at).0 {
        return LiteralClassification::NotPrimitive;
    }

    // Keep typed bytes separate from decoded text, even though both are constant expressions.
    let scalar_kind = payload(tree, at, SCALAR_KIND);
    if scalar_kind.map(|each| each.0) == Some(BYTES_SCALAR) {
        return payload(tree, at, SCALAR_VALUE).map_or(
            LiteralClassification::MissingPayload,
            |value| {
                flag(
                    NameText(crate::domain::syntax::constants::BYTES_SCALAR),
                    Flag::Lowered,
                    if value.0.is_empty() {
                        Flag::Raised
                    } else {
                        Flag::Lowered
                    },
                    Flag::Lowered,
                )
            },
        );
    }

    if literal(tree, at) == Literal::String {
        return decoded_string_empty(tree, at).map_or(
            LiteralClassification::MissingPayload,
            |empty| {
                flag(
                    NameText(constants::STRING),
                    Flag::Lowered,
                    empty,
                    Flag::Lowered,
                )
            },
        );
    }

    let Some(scalar_kind) = scalar_kind else {
        return LiteralClassification::MissingPayload;
    };
    match scalar_kind.0 {
        INTEGER_SCALAR => {
            payload(tree, at, SCALAR_VALUE).map_or(LiteralClassification::MissingPayload, |value| {
                flag(
                    NameText(crate::domain::syntax::constants::INTEGER_SCALAR),
                    if value.0 == constants::ZERO_PAYLOAD {
                        Flag::Raised
                    } else {
                        Flag::Lowered
                    },
                    Flag::Lowered,
                    Flag::Lowered,
                )
            })
        }
        FLOAT_SCALAR => payload(tree, at, SCALAR_VALUE)
            .and_then(|bits| match bits.0.parse::<u64>() {
                Ok(parsed) => Some(parsed),
                Err(_) => None,
            })
            .map_or(LiteralClassification::MissingPayload, |bits| {
                flag(
                    NameText(crate::domain::syntax::constants::FLOAT_SCALAR),
                    if f64::from_bits(bits) == 0.0_f64 {
                        Flag::Raised
                    } else {
                        Flag::Lowered
                    },
                    Flag::Lowered,
                    Flag::Lowered,
                )
            }),
        COMPLEX_SCALAR => payload(tree, at, SCALAR_VALUE)
            .and_then(|real| match real.0.parse::<u64>() {
                Ok(parsed) => Some(parsed),
                Err(_) => None,
            })
            .zip(payload(tree, at, SCALAR_IMAGINARY).and_then(|imaginary| {
                match imaginary.0.parse::<u64>() {
                    Ok(parsed) => Some(parsed),
                    Err(_) => None,
                }
            }))
            .map_or(
                LiteralClassification::MissingPayload,
                |(real, imaginary)| {
                    flag(
                        NameText(crate::domain::syntax::constants::COMPLEX_SCALAR),
                        if f64::from_bits(real) == 0.0_f64 && f64::from_bits(imaginary) == 0.0_f64 {
                            Flag::Raised
                        } else {
                            Flag::Lowered
                        },
                        Flag::Lowered,
                        Flag::Lowered,
                    )
                },
            ),
        BOOLEAN_SCALAR => payload(tree, at, SCALAR_VALUE)
            .and_then(|value| match value.0.parse::<bool>() {
                Ok(parsed) => Some(parsed),
                Err(_) => None,
            })
            .map_or(LiteralClassification::MissingPayload, |boolean| {
                flag(
                    NameText(crate::domain::syntax::constants::BOOLEAN_SCALAR),
                    Flag::Lowered,
                    Flag::Lowered,
                    if boolean { Flag::Raised } else { Flag::Lowered },
                )
            }),
        NONE_SCALAR => flag(
            NameText(constants::NULL),
            Flag::Lowered,
            Flag::Lowered,
            Flag::Lowered,
        ),
        ELLIPSIS_SCALAR => flag(
            NameText(crate::domain::syntax::constants::ELLIPSIS_SCALAR),
            Flag::Lowered,
            Flag::Lowered,
            Flag::Lowered,
        ),
        _ => LiteralClassification::MissingPayload,
    }
}
