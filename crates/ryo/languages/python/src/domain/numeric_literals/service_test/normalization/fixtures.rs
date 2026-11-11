//! The support of the group's tests.
use super::super::super::{
    service::{classify, write_float},
    types::{FloatBits, FloatBuffer, NumericClassification, NumericValue},
};
use crate::domain::semantic_syntax::service::positions;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

pub(super) struct NumericLiteral {
    /// The literal's numeric kind.
    pub(super) kind: &'static str,
    /// Its normalized value, as text.
    pub(super) value: String,
}

pub(super) fn float_text(bits: u64) -> String {
    let mut buffer = FloatBuffer([0; crate::domain::numeric_literals::constants::FLOAT_TEXT_BYTES]);
    write_float(FloatBits(bits), &mut buffer).0.to_owned()
}

pub(super) fn literals(source: &'static str) -> Vec<NumericLiteral> {
    let file = parsed(source.as_bytes()).expect("literal examples parse");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    positions(&tree)
        .filter_map(|at| match classify(&tree, at) {
            NumericClassification::Known(value) => Some(NumericLiteral {
                kind: value.kind.0,
                value: match value.value {
                    NumericValue::Integer(text) => text.0.to_owned(),
                    NumericValue::Float(bits) => float_text(bits.0),
                },
            }),
            NumericClassification::NotNumeric => None,
            NumericClassification::MissingPayload => {
                panic!("the parser supplied a numeric payload")
            }
        })
        .collect()
}
