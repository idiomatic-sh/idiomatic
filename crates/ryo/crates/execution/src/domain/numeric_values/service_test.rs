//! Precision boundaries are specified independently of the implementation's representation.

use super::{
    service::{compare, read},
    types::Numeric,
};
use std::cmp::Ordering::{Equal, Greater, Less};

#[test]
fn integer_float_and_wide_mixed_ordering_is_exact() {
    let examples = [
        ("0", "-0", Equal),
        ("0", "-0.0", Equal),
        ("1", "1.0", Equal),
        ("-2", "-1", Less),
        ("100000000000000000000", "99999999999999999999", Greater),
        ("9007199254740993", "9007199254740992", Greater),
        ("9007199254740993", "9007199254740992.0", Greater),
        ("-9007199254740993", "-9007199254740992.0", Less),
        ("18446744073709551615", "1.8446744073709552e19", Less),
        ("18446744073709551617", "1.8446744073709552e19", Greater),
        (
            "340282366920938463463374607431768211455",
            "3.402823669209385e38",
            Less,
        ),
        (
            "340282366920938463463374607431768211456",
            "3.402823669209385e38",
            Equal,
        ),
        (
            "340282366920938463463374607431768211457",
            "3.402823669209385e38",
            Greater,
        ),
        (
            "-340282366920938463463374607431768211457",
            "-3.402823669209385e38",
            Less,
        ),
        ("340282366920938463463374607431768211456", "1.0", Greater),
        ("0", "5e-324", Less),
        ("1", "5e-324", Greater),
        ("-1", "-5e-324", Less),
        ("0", "2.2250738585072014e-308", Less),
        ("1", "0.9999999999999999", Greater),
        ("1", "1.0000000000000002", Less),
        ("86400.25", "86400", Greater),
        ("86400.25", "86401", Less),
        ("inf", "1.7976931348623157e308", Greater),
        ("-inf", "-99999999999999999999999999999999999", Less),
        ("inf", "inf", Equal),
        ("-inf", "-inf", Equal),
    ];
    for (left, right, expected) in examples {
        let left_value = read(left).expect("canonical numeric example");
        let right_value = read(right).expect("canonical numeric example");
        assert_eq!(
            compare(left_value, right_value),
            Some(expected),
            "{left} versus {right}"
        );
        assert_eq!(
            compare(right_value, left_value),
            Some(expected.reverse()),
            "reverse {right} versus {left}"
        );
    }
    let wider_than_binary64 = (num_bigint::BigUint::from(1_u8) << 1024_u32).to_string();
    let integer = read(&wider_than_binary64).expect("canonical wide integer");
    assert_eq!(compare(integer, Numeric::Float(f64::MAX)), Some(Greater));
    assert_eq!(compare(integer, Numeric::Float(f64::INFINITY)), Some(Less));
}

#[test]
fn malformed_source_spellings_and_nan_never_form_an_ordered_pair() {
    for text in [
        "", " 1", "1 ", "+1", "01", "1_000", "0xff", "0o10", "0b10", "1j", "True", "--1", "1.",
        ".5", "1e", "1e+", "1.0x", "00.0", "Infinity",
    ] {
        assert_eq!(read(text), None, "{text:?} is not normalized numeric text");
    }
    let nan = read("NaN").expect("explicit NaN");
    for other in [Numeric::Whole(0), Numeric::Float(f64::INFINITY), nan] {
        assert_eq!(compare(nan, other), None);
        assert_eq!(compare(other, nan), None);
    }
}

#[test]
fn existing_whole_numbers_do_not_round_through_float() {
    let largest = Numeric::Whole(i64::MAX);
    assert_eq!(
        compare(largest, read("9223372036854775807").expect("integer")),
        Some(Equal)
    );
    assert_eq!(
        compare(largest, read("9223372036854775808").expect("integer")),
        Some(Less)
    );
    assert_eq!(
        compare(largest, read("9223372036854775808.0").expect("float")),
        Some(Less)
    );
    assert_eq!(
        compare(
            Numeric::Whole(i64::MIN),
            read("-9223372036854775808").expect("integer")
        ),
        Some(Equal)
    );
    assert_eq!(
        compare(Numeric::Whole(-1), read("-1.0").expect("float")),
        Some(Equal)
    );
    assert_eq!(
        compare(Numeric::Whole(0), read("-1").expect("integer")),
        Some(Greater)
    );
}

#[test]
fn integer_successor_preserves_sign_precision_and_wide_boundaries() {
    use super::service::integer_successor;
    for (left, right) in [
        ("-2", "-1"),
        ("-1", "0"),
        ("-0", "1"),
        ("42", "43"),
        ("9007199254740992", "9007199254740993"),
        ("9223372036854775807", "9223372036854775808"),
        (
            "170141183460469231731687303715884105727",
            "170141183460469231731687303715884105728",
        ),
        (
            "-170141183460469231731687303715884105728",
            "-170141183460469231731687303715884105727",
        ),
    ] {
        let left = read(left).expect("integer");
        let right = read(right).expect("integer");
        assert!(integer_successor(left, right));
        assert!(!integer_successor(right, left));
        assert!(!integer_successor(left, left));
    }
    for (left, right) in [
        ("1.0", "2"),
        ("1", "2.0"),
        ("inf", "2"),
        ("1", "NaN"),
        ("42", "44"),
    ] {
        assert!(!integer_successor(
            read(left).expect("numeric"),
            read(right).expect("numeric")
        ));
    }
    assert!(integer_successor(
        Numeric::Whole(i64::MAX),
        read("9223372036854775808").expect("wide integer")
    ));
    assert!(integer_successor(
        Numeric::Whole(i64::MIN),
        read("-9223372036854775807").expect("integer")
    ));
}
