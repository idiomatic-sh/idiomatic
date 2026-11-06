//! Optional decoded bytes; source spelling is never parsed by this reader.
use crate::domain::{
    literal_properties::{service::classify, types::LiteralClassification},
    model::types::Declarations,
    nodes::types::Node,
    scopes::service::owner,
    semantic_syntax::types::Visibility,
    syntax::{constants::SCALAR_VALUE, service::kind_bits, types::Kind},
};
use ryo::languages::{self, Flag, NameText};
use super::constants::INCOMPLETE;
/// Optional reader for original byte-string occurrences.
pub(crate) struct BytesLiteralItem;
/// Canonical lossless octet encoding, including empty bytes.
fn canonical(value: NameText<'_>) -> Flag {
    if value.0.len() % 2 == 0
        && value
            .0
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Flag::Raised
    } else {
        Flag::Lowered
    }
}
/// A typed payload, a non-byte occurrence, or incomplete scalar metadata.
enum Decoded<'a> {
    Other,
    Missing,
    Bytes(NameText<'a>),
}
fn decoded<'a>(tree: &'a languages::Tree<'_>, at: languages::Position) -> Decoded<'a> {
    // The shared primitive classifier excludes synthetic interpolation fragments.
    // Missing primitive payload stays unknown, without mistaking literal text for bytes.
    match classify(tree, at) {
        LiteralClassification::NotPrimitive => Decoded::Other,
        LiteralClassification::MissingPayload => Decoded::Missing,
        LiteralClassification::Known(properties) if properties.kind.0 == crate::domain::syntax::constants::BYTES_SCALAR => {
            match languages::property_at(tree.elements, at, SCALAR_VALUE) {
                Some(value) if canonical(value) == Flag::Raised => Decoded::Bytes(value),
                Some(_) | None => Decoded::Missing,
            }
        }
        LiteralClassification::Known(_) => Decoded::Other,
    }
}
ryo::reads! {
    BytesLiteralItem => languages::BytesLiteral, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        each (crate::domain::semantic_syntax::service::positions(tree)) as expression {
            require crate::domain::semantic_syntax::service::visibility(tree, model, expression, symbols) == Visibility::Visible;
            when decoded(tree, expression) {
                Decoded::Other => {}
                Decoded::Missing => { partial INCOMPLETE => languages::Missing::Any; }
                Decoded::Bytes(hex) => {
                    given value = languages::intern_derived(symbols, tree.source, hex);
                    row { occurrence: node(expression), owner: value(owner(tree, expression)), hex_value: value(value), file: file, span: span(expression), }
                }
            }
        }
    }
}
