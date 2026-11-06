//! Parser values normalize every Python radix without evaluating unary or arithmetic expressions.

use crate::cases::{Case, CaseName, Expect, Word, run};
use super::{
    service::{classify, float_text},
    types::{Classification, NumericValue},
};
use crate::domain::semantic_syntax::service::positions;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

struct NumericLiteral {
    kind: &'static str,
    value: String,
}

fn literals(source: &'static str) -> Vec<NumericLiteral> {
    let file = parsed(source.as_bytes()).expect("literal examples parse");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    positions(&tree)
        .filter_map(|at| match classify(&tree, at) {
            Classification::Known(value) => Some(NumericLiteral {
                kind: value.kind,
                value: match value.value {
                    NumericValue::Integer(text) => text.to_owned(),
                    NumericValue::Float(bits) => float_text(bits),
                },
            }),
            Classification::NotNumeric => None,
            Classification::MissingPayload => panic!("the parser supplied a numeric payload"),
        })
        .collect()
}

fn integer_payloads_are_canonical_arbitrary_precision_and_exclude_other_primitives() {
    let values = literals(
        "0x10\n0o20\n0b10000\n1_000\n9007199254740993\n18446744073709551616\n340282366920938463463374607431768211457\nTrue\nFalse\n1j\nNone\n'42'\nb'42'\n",
    );
    assert_eq!(
        values
            .iter()
            .map(|value| (value.kind, value.value.as_str()))
            .collect::<Vec<_>>(),
        [
            ("integer", "16"),
            ("integer", "16"),
            ("integer", "16"),
            ("integer", "1000"),
            ("integer", "9007199254740993"),
            ("integer", "18446744073709551616"),
            ("integer", "340282366920938463463374607431768211457"),
        ]
    );
}

fn unary_signs_and_arithmetic_remain_separate_syntax() {
    assert_eq!(
        literals("-42\n+0x10\n1 + 2\n-0.0\n")
            .iter()
            .map(|value| value.value.as_str())
            .collect::<Vec<_>>(),
        ["42", "16", "1", "2", "0.0"]
    );
}

fn floats_round_trip_and_keep_their_numeric_kind() {
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
        assert_eq!(value.kind, "float");
        assert_eq!(
            value
                .value
                .parse::<f64>()
                .expect("round-trip text")
                .to_bits(),
            expected.to_bits()
        );
        assert!(!expected.is_finite() || value.value.contains(['.', 'e', 'E']));
    }
    assert_eq!(values.len(), expected.len());
    assert_eq!(float_text((-0.0_f64).to_bits()), "-0.0");
    assert_eq!(float_text(f64::NEG_INFINITY.to_bits()), "-inf");
    assert_eq!(float_text(f64::NAN.to_bits()), "NaN");
    for bits in [
        1_u64,
        1_u64 | (1_u64 << 63),
        f64::MAX.to_bits(),
        f64::MIN.to_bits(),
    ] {
        let text = float_text(bits);
        assert!(text.len() <= 350);
        assert_eq!(
            text.parse::<f64>()
                .expect("finite float spelling")
                .to_bits(),
            bits
        );
    }
}

fn literals_inside_interpolations_remain_real_numeric_occurrences() {
    assert_eq!(
        literals("f'1{0x10}2{1.0}'\n")
            .iter()
            .map(|value| value.value.as_str())
            .collect::<Vec<_>>(),
        ["16", "1.0"]
    );
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("integer_payloads_are_canonical_arbitrary_precision_and_exclude_other_primitives"),
        word: Word::Succeeds,
        input: || integer_payloads_are_canonical_arbitrary_precision_and_exclude_other_primitives,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unary_signs_and_arithmetic_remain_separate_syntax"),
        word: Word::Succeeds,
        input: || unary_signs_and_arithmetic_remain_separate_syntax,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("floats_round_trip_and_keep_their_numeric_kind"),
        word: Word::Succeeds,
        input: || floats_round_trip_and_keep_their_numeric_kind,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("literals_inside_interpolations_remain_real_numeric_occurrences"),
        word: Word::Succeeds,
        input: || literals_inside_interpolations_remain_real_numeric_occurrences,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
