//! The support of the group's tests.

use super::super::super::{
    service::classify,
    types::{NumericClassification, NumericValue},
};
use crate::domain::semantic_syntax::service::positions;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, FLOAT_TEXT_BYTES, FloatBits, FloatBuffer, Tree};

pub(super) struct NumericLiteral {
    /// The literal's numeric kind.
    pub(super) kind: &'static str,
    /// Its normalized value, as text.
    pub(super) value: String,
}

pub(super) fn float_text(bits: u64) -> String {
    let mut buffer = FloatBuffer([0; FLOAT_TEXT_BYTES]);
    languages::binary64_text(FloatBits(bits), &mut buffer)
        .0
        .to_owned()
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
            NumericClassification::MissingPayload => Some(NumericLiteral {
                kind: "MissingPayload",
                value: "MissingPayload".to_owned(),
            }),
        })
        .collect()
}

pub(super) fn integer_payloads_are_canonical_arbitrary_precision_and_exclude_other_primitives()
-> String {
    let mut observed: Vec<String> = Vec::new();
    let values = literals(
        "0x10\n0o20\n0b10000\n1_000\n9007199254740993\n18446744073709551616\n340282366920938463463374607431768211457\nTrue\nFalse\n1j\nNone\n'42'\nb'42'\n",
    );
    observed.push(format!(
        "{:?}",
        values
            .iter()
            .map(|value| (value.kind, value.value.as_str()))
            .collect::<Vec<_>>()
    ));
    observed.join("|")
}

pub(super) fn unary_signs_and_arithmetic_remain_separate_syntax() -> String {
    let mut observed: Vec<String> = Vec::new();
    observed.push(format!(
        "{:?}",
        literals("-42\n+0x10\n1 + 2\n-0.0\n")
            .iter()
            .map(|value| value.value.as_str())
            .collect::<Vec<_>>()
    ));
    observed.join("|")
}

pub(super) fn floats_round_trip_and_keep_their_numeric_kind() -> String {
    let mut observed: Vec<String> = Vec::new();
    let values = literals("1.0\n1e3\n1_000.0\n5e-324\n1e-9999\n1e9999\n");
    let expected = [
        1.0_f64,
        1000.0,
        1000.0,
        f64::from_bits(1),
        0.0,
        f64::INFINITY,
    ];
    for (value, expected) in values.iter().zip(expected) {
        observed.push(format!("{:?}", value.kind));
        observed.push(format!(
            "{:?}",
            value.value.parse::<f64>().map(f64::to_bits)
        ));
        observed.push(format!(
            "{}",
            !expected.is_finite() || value.value.contains(['.', 'e', 'E'])
        ));
    }
    observed.push(format!("{:?}", values.len()));
    observed.push(format!("{:?}", float_text((-0.0_f64).to_bits())));
    observed.push(format!("{:?}", float_text(f64::NEG_INFINITY.to_bits())));
    observed.push(format!("{:?}", float_text(f64::NAN.to_bits())));
    for bits in [
        1_u64,
        1_u64 | (1_u64 << 63),
        f64::MAX.to_bits(),
        f64::MIN.to_bits(),
    ] {
        let text = float_text(bits);
        observed.push(format!("{}", text.len() <= 350));
        observed.push(format!("{:?}", text.parse::<f64>().map(f64::to_bits)));
    }
    observed.join("|")
}

pub(super) fn literals_inside_interpolations_remain_real_numeric_occurrences() -> String {
    let mut observed: Vec<String> = Vec::new();
    observed.push(format!(
        "{:?}",
        literals("f'1{0x10}2{1.0}'\n")
            .iter()
            .map(|value| value.value.as_str())
            .collect::<Vec<_>>()
    ));
    observed.join("|")
}
