//! Parser values normalize every Python radix without evaluating unary or arithmetic expressions.
use super::fixtures::{float_text, literals};

#[test]
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

#[test]
fn unary_signs_and_arithmetic_remain_separate_syntax() {
    assert_eq!(
        literals("-42\n+0x10\n1 + 2\n-0.0\n")
            .iter()
            .map(|value| value.value.as_str())
            .collect::<Vec<_>>(),
        ["42", "16", "1", "2", "0.0"]
    );
}

#[test]
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

#[test]
fn literals_inside_interpolations_remain_real_numeric_occurrences() {
    assert_eq!(
        literals("f'1{0x10}2{1.0}'\n")
            .iter()
            .map(|value| value.value.as_str())
            .collect::<Vec<_>>(),
        ["16", "1.0"]
    );
}
