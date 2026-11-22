//! The `literal_facts` service: free functions over the literals of the parser's tree. Every answer comes from the parser's own
//! decoded payload (a number's value, a string's parts), never from reading the source spelling again; only the exactness of a
//! float compares the digits written with the value.

use super::types::{LiteralPiece, NumberFacts, StringValue, TextBuffer};
use crate::domain::fact_notes::constants::LOADERS;
use crate::domain::literal_properties::constants::{NULL, STRING};
use crate::domain::literal_properties::types::Properties;
use crate::domain::node::service::{element_id, id_of, node_at, written};
use crate::domain::node::types::SyntaxFile;
use crate::domain::syntax::constants::{
    BOOLEAN_SCALAR, BYTES_SCALAR, COMPLEX_SCALAR, ELLIPSIS_SCALAR, FLOAT_SCALAR, INTEGER_SCALAR,
};
use crate::domain::syntax::service::kind_of;
use crate::domain::syntax::types::{Interpolation, Kind};
use crate::domain::tokens::types::Token;
use ruff_python_ast::{
    AnyNodeRef, ArgOrKeyword, FStringPartRef, InterpolatedStringElement,
    InterpolatedStringElements, Number,
};
use ryo::languages::{
    self, FLOAT_TEXT_BYTES, Flag, FloatBits, FloatBuffer, NameText, Position, WRITTEN_DIGITS_BYTES,
    node_table,
};
use std::fmt::Write as _;

/// The properties of a primitive literal of the kind named.
fn known(kind: NameText<'static>, zero: Flag, empty: Flag, boolean: Flag) -> Option<Properties> {
    Some(Properties {
        kind,
        zero,
        empty,
        boolean,
    })
}

/// Whether a float is zero, of either sign.
const fn zero(bits: FloatBits) -> Flag {
    if bits.0 << 1 == 0 {
        Flag::Raised
    } else {
        Flag::Lowered
    }
}

/// Raised when both flags are.
const fn both(first: Flag, second: Flag) -> Flag {
    match (first, second) {
        (Flag::Raised, Flag::Raised) => Flag::Raised,
        (Flag::Raised | Flag::Lowered, Flag::Raised | Flag::Lowered) => Flag::Lowered,
    }
}

/// The kind of a primitive literal and whether it is zero, empty or true, from the parser's payload; none for any other node.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the six primitive literals have properties; every other node is no primitive literal"
)]
#[must_use]
pub(crate) fn properties(node: AnyNodeRef<'_>) -> Option<Properties> {
    let (no, yes) = (Flag::Lowered, Flag::Raised);
    match node {
        AnyNodeRef::ExprStringLiteral(literal) => {
            let empty = literal.value.iter().all(|part| part.value.is_empty());
            known(NameText(STRING), no, if empty { yes } else { no }, no)
        }
        AnyNodeRef::ExprBytesLiteral(literal) => {
            let empty = literal.value.iter().all(|part| part.value.is_empty());
            known(NameText(BYTES_SCALAR), no, if empty { yes } else { no }, no)
        }
        AnyNodeRef::ExprNumberLiteral(literal) => match &literal.value {
            Number::Int(value) => known(
                NameText(INTEGER_SCALAR),
                if value.as_u64() == Some(0) { yes } else { no },
                no,
                no,
            ),
            Number::Float(value) => known(
                NameText(FLOAT_SCALAR),
                zero(FloatBits(value.to_bits())),
                no,
                no,
            ),
            Number::Complex { real, imag } => known(
                NameText(COMPLEX_SCALAR),
                both(
                    zero(FloatBits(real.to_bits())),
                    zero(FloatBits(imag.to_bits())),
                ),
                no,
                no,
            ),
        },
        AnyNodeRef::ExprBooleanLiteral(literal) => known(
            NameText(BOOLEAN_SCALAR),
            no,
            no,
            if literal.value { yes } else { no },
        ),
        AnyNodeRef::ExprNoneLiteral(_) => known(NameText(NULL), no, no, no),
        AnyNodeRef::ExprEllipsisLiteral(_) => known(NameText(ELLIPSIS_SCALAR), no, no, no),
        _ => None,
    }
}

/// Whether the digits a float is written with are the value's: the digits before an exponent, without separators or leading
/// zeros, read back as the same binary64.
fn exact(spelling: NameText<'_>, bits: FloatBits) -> Flag {
    let mut digits = [0_u8; WRITTEN_DIGITS_BYTES];
    let mut length = 0;
    for byte in spelling
        .0
        .bytes()
        .take_while(|byte| !matches!(byte, b'e' | b'E'))
        .filter(|byte| *byte != b'_')
        .skip_while(|byte| *byte == b'0')
    {
        let Some(slot) = digits.get_mut(length) else {
            return Flag::Lowered;
        };
        *slot = byte;
        length += 1;
    }
    match digits.get(..length).map(std::str::from_utf8) {
        Some(Ok(text)) => languages::exact_digits(NameText(text), bits),
        // digits copied from a float's spelling are ASCII; a cut that is not text writes no digits to give back
        Some(Err(_)) | None => Flag::Lowered,
    }
}

/// What the number at `id` is, its value written into `value`: an integer's decimal digits, a float's binary64 text. None for a
/// complex number and for any other node.
pub(crate) fn number(
    file: &SyntaxFile<'_>,
    id: Position,
    value: &mut TextBuffer,
) -> Option<NumberFacts> {
    let AnyNodeRef::ExprNumberLiteral(literal) = node_at(file, id)? else {
        return None;
    };
    value.0.clear();
    match &literal.value {
        Number::Int(integer) => {
            if write!(value.0, "{integer}").is_err() {
                return None;
            }
            let digits = value.0.as_str();
            let canonical = !digits.is_empty()
                && digits.bytes().all(|byte| byte.is_ascii_digit())
                && (digits.len() == 1 || !digits.starts_with('0'));
            canonical.then_some(NumberFacts {
                kind: NameText(INTEGER_SCALAR),
                exact: Flag::Raised,
            })
        }
        Number::Float(float) => {
            let bits = FloatBits(float.to_bits());
            let mut buffer = FloatBuffer([0; FLOAT_TEXT_BYTES]);
            value
                .0
                .push_str(languages::binary64_text(bits, &mut buffer).0);
            Some(NumberFacts {
                kind: NameText(FLOAT_SCALAR),
                exact: exact(written(file, id), bits),
            })
        }
        Number::Complex { .. } => None,
    }
}

/// Joins the literal elements of an f-string's or a t-string's part into `joined`; says whether one of them interpolates.
fn joined(elements: &InterpolatedStringElements, joined: &mut TextBuffer) -> Interpolation {
    for element in elements {
        match element {
            InterpolatedStringElement::Literal(literal) => joined.0.push_str(&literal.value),
            InterpolatedStringElement::Interpolation(_) => return Interpolation::Interpolates,
        }
    }
    Interpolation::Static
}

/// What the string, f-string or t-string `node` holds: its decoded parts joined (through `buffer` when it has more than one), or
/// that it interpolates. None for any other node.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the three kinds of string hold a string value; every other node holds none"
)]
pub(crate) fn string_value<'b>(
    node: AnyNodeRef<'b>,
    buffer: &'b mut TextBuffer,
) -> Option<StringValue<'b>> {
    buffer.0.clear();
    match node {
        AnyNodeRef::ExprStringLiteral(literal) => {
            if literal.value.is_implicit_concatenated() {
                for part in &literal.value {
                    buffer.0.push_str(&part.value);
                }
                Some(StringValue::Known(NameText(buffer.0.as_str())))
            } else {
                Some(StringValue::Known(NameText(literal.value.to_str())))
            }
        }
        AnyNodeRef::ExprFString(string) => {
            for part in &string.value {
                let part_is = match part {
                    FStringPartRef::Literal(literal) => {
                        buffer.0.push_str(&literal.value);
                        Interpolation::Static
                    }
                    FStringPartRef::FString(formatted) => joined(&formatted.elements, buffer),
                };
                if part_is == Interpolation::Interpolates {
                    return Some(StringValue::Interpolated);
                }
            }
            Some(StringValue::Known(NameText(buffer.0.as_str())))
        }
        AnyNodeRef::ExprTString(string) => {
            for part in &string.value {
                if joined(&part.elements, buffer) == Interpolation::Interpolates {
                    return Some(StringValue::Interpolated);
                }
            }
            Some(StringValue::Known(NameText(buffer.0.as_str())))
        }
        _ => None,
    }
}

/// What the format spec of an interpolation holds: its literal text, or that it interpolates. None for any other node.
pub(crate) fn format_spec_value<'b>(
    node: AnyNodeRef<'b>,
    buffer: &'b mut TextBuffer,
) -> Option<StringValue<'b>> {
    let AnyNodeRef::InterpolatedStringFormatSpec(spec) = node else {
        return None;
    };
    buffer.0.clear();
    // Every literal piece is read before the answer: a spec that interpolates anywhere holds no text a file states.
    let mut interpolates = Interpolation::Static;
    for element in &spec.elements {
        match element {
            InterpolatedStringElement::Literal(literal) => buffer.0.push_str(&literal.value),
            InterpolatedStringElement::Interpolation(_) => {
                interpolates = Interpolation::Interpolates;
            }
        }
    }
    Some(match interpolates {
        Interpolation::Interpolates => StringValue::Interpolated,
        Interpolation::Static => StringValue::Known(NameText(buffer.0.as_str())),
    })
}

/// The string tokens of the string `node` into `tokens`, in the order they are written: one for each part of an implicit
/// concatenation. Nothing for a node that is no string.
pub(crate) fn string_tokens(
    file: &SyntaxFile<'_>,
    node: AnyNodeRef<'_>,
    tokens: &mut Vec<Position>,
) {
    tokens.clear();
    let AnyNodeRef::ExprStringLiteral(literal) = node else {
        return;
    };
    for part in &literal.value {
        let Some(part_id) = element_id(file, part) else {
            continue;
        };
        tokens.extend(
            node_table::children_of(file.table, part_id)
                .iter()
                .copied()
                .filter(|token| {
                    kind_of(node_table::kind_of(file.table, *token)) == Kind::Token(Token::String)
                }),
        );
    }
}

/// Whether the string at `id` names a module a call loads: the first argument of `__import__(..)` or of
/// `importlib.import_module(..)`, the callee as it is written.
#[must_use]
pub(crate) fn specifier(file: &SyntaxFile<'_>, id: Position) -> Flag {
    let arguments = node_table::parent_of(file.table, id);
    let Some(AnyNodeRef::Arguments(typed)) = node_at(file, arguments) else {
        return Flag::Lowered;
    };
    let call = node_table::parent_of(file.table, arguments);
    let Some(AnyNodeRef::ExprCall(calling)) = node_at(file, call) else {
        return Flag::Lowered;
    };
    let first = typed
        .iter_source_order()
        .next()
        .and_then(|argument| match argument {
            ArgOrKeyword::Arg(expression) => element_id(file, expression),
            ArgOrKeyword::Keyword(keyword) => element_id(file, keyword),
        });
    let loads = element_id(file, calling.func.as_ref())
        .is_some_and(|callee| LOADERS.contains(&written(file, callee).0));
    if first == Some(id) && loads {
        Flag::Raised
    } else {
        Flag::Lowered
    }
}

/// The value of a bytes literal as lowercase hexadecimal octets, written into `hex`; nothing for any other node.
pub(crate) fn bytes_hex(node: AnyNodeRef<'_>, hex: &mut TextBuffer) -> Option<()> {
    let AnyNodeRef::ExprBytesLiteral(literal) = node else {
        return None;
    };
    hex.0.clear();
    for byte in literal.value.bytes() {
        if write!(hex.0, "{byte:02x}").is_err() {
            return None;
        }
    }
    Some(())
}

/// The literal pieces of the f-string `node`, in the order they are written: its plain string parts and the literal text between
/// the interpolations of its f-string parts (a format spec's pieces are its interpolation's, not the string's).
pub(crate) fn fragments<'n>(node: AnyNodeRef<'n>, out: &mut Vec<LiteralPiece<'n>>) {
    let AnyNodeRef::ExprFString(string) = node else {
        return;
    };
    for part in &string.value {
        match part {
            FStringPartRef::Literal(literal) => {
                if let Some(id) = id_of(literal) {
                    out.push(LiteralPiece {
                        id,
                        value: NameText(&literal.value),
                    });
                }
            }
            FStringPartRef::FString(formatted) => {
                for element in &formatted.elements {
                    if let InterpolatedStringElement::Literal(literal) = element
                        && let Some(id) = id_of(literal)
                    {
                        out.push(LiteralPiece {
                            id,
                            value: NameText(&literal.value),
                        });
                    }
                }
            }
        }
    }
}
