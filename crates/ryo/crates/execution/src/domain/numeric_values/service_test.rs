//! Precision boundaries are specified independently of the implementation's representation.

use super::{
    service::{compare, integer_successor, read},
    types::{FloatValue, Numeric, NumberText, Succession},
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::WholeNumber;
use std::cmp::Ordering::{self, Equal, Greater, Less};

/// What a row asks of the part.
enum Scenario {
    /// Pairs of text and the exact order of each, in both directions.
    Ordered(&'static [(&'static str, &'static str, Ordering)]),
    /// An integer of 2^1024 against the largest and the infinite binary64.
    WiderThanBinary64,
    /// Text that is not normalized numeric text.
    Unreadable(&'static [&'static str]),
    /// NaN against every other kind of operand.
    NotANumber,
    /// Existing whole numbers against decimal and float text.
    Whole,
    /// Pairs of integer text where the right is the left plus one.
    Successors(&'static [(&'static str, &'static str)]),
    /// Pairs of numeric text where the right is not the left plus one.
    NotSuccessors(&'static [(&'static str, &'static str)]),
    /// Existing whole numbers at the edges of their range.
    WholeSuccessors,
}
/// A scenario: what a row asks of the part.
type Input = Scenario;
/// How many checks of the row held.
type Value = usize;
/// Nothing in this part refuses.
type Failure = std::convert::Infallible;

const ORDERED: &[(&str, &str, Ordering)] = &[
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

const UNREADABLE: &[&str] = &[
    "", " 1", "1 ", "+1", "01", "1_000", "0xff", "0o10", "0b10", "1j", "True", "--1", "1.", ".5",
    "1e", "1e+", "1.0x", "00.0", "Infinity",
];

const SUCCESSORS: &[(&str, &str)] = &[
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
];

const NOT_SUCCESSORS: &[(&str, &str)] = &[
    ("1.0", "2"),
    ("1", "2.0"),
    ("inf", "2"),
    ("1", "NaN"),
    ("42", "44"),
];

fn numeric(text: &str) -> Numeric<'_> {
    read(NumberText(text)).expect("canonical numeric example")
}

fn ordered(examples: &[(&str, &str, Ordering)]) -> Value {
    for (left, right, expected) in examples {
        let (left_value, right_value) = (numeric(left), numeric(right));
        assert_eq!(
            compare(left_value, right_value),
            Some(*expected),
            "{left} versus {right}"
        );
        assert_eq!(
            compare(right_value, left_value),
            Some(expected.reverse()),
            "reverse {right} versus {left}"
        );
    }
    examples.len()
}

fn wider_than_binary64() -> Value {
    let wider = (num_bigint::BigUint::from(1_u8) << 1024_u32).to_string();
    let integer = numeric(&wider);
    assert_eq!(
        compare(integer, Numeric::Float(FloatValue(f64::MAX))),
        Some(Greater)
    );
    assert_eq!(
        compare(integer, Numeric::Float(FloatValue(f64::INFINITY))),
        Some(Less)
    );
    2
}

fn unreadable(texts: &[&str]) -> Value {
    for text in texts {
        assert_eq!(
            read(NumberText(text)),
            None,
            "{text:?} is not normalized numeric text"
        );
    }
    texts.len()
}

fn not_a_number() -> Value {
    let nan = numeric("NaN");
    let others = [
        Numeric::Whole(WholeNumber(0)),
        Numeric::Float(FloatValue(f64::INFINITY)),
        nan,
    ];
    for other in others {
        assert_eq!(compare(nan, other), None);
        assert_eq!(compare(other, nan), None);
    }
    others.len()
}

fn whole() -> Value {
    let largest = Numeric::Whole(WholeNumber(i64::MAX));
    let checks = [
        (largest, numeric("9223372036854775807"), Equal),
        (largest, numeric("9223372036854775808"), Less),
        (largest, numeric("9223372036854775808.0"), Less),
        (
            Numeric::Whole(WholeNumber(i64::MIN)),
            numeric("-9223372036854775808"),
            Equal,
        ),
        (Numeric::Whole(WholeNumber(-1)), numeric("-1.0"), Equal),
        (Numeric::Whole(WholeNumber(0)), numeric("-1"), Greater),
    ];
    for (left, right, expected) in checks {
        assert_eq!(compare(left, right), Some(expected));
    }
    checks.len()
}

fn successors(pairs: &[(&str, &str)]) -> Value {
    for (left, right) in pairs {
        let (left, right) = (numeric(left), numeric(right));
        assert_eq!(integer_successor(left, right), Succession::Successor);
        assert_eq!(integer_successor(right, left), Succession::NotSuccessor);
        assert_eq!(integer_successor(left, left), Succession::NotSuccessor);
    }
    pairs.len()
}

fn not_successors(pairs: &[(&str, &str)]) -> Value {
    for (left, right) in pairs {
        assert_eq!(
            integer_successor(numeric(left), numeric(right)),
            Succession::NotSuccessor
        );
    }
    pairs.len()
}

fn whole_successors() -> Value {
    assert_eq!(
        integer_successor(
            Numeric::Whole(WholeNumber(i64::MAX)),
            numeric("9223372036854775808")
        ),
        Succession::Successor
    );
    assert_eq!(
        integer_successor(
            Numeric::Whole(WholeNumber(i64::MIN)),
            numeric("-9223372036854775807")
        ),
        Succession::Successor
    );
    2
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("integer_float_and_wide_mixed_ordering_is_exact"),
        word: Word::Succeeds,
        input: || Scenario::Ordered(ORDERED),
        expect: Expect::Value(27),
    },
    Case {
        name: CaseName("a_wider_integer_than_binary64_orders_against_its_extremes"),
        word: Word::Succeeds,
        input: || Scenario::WiderThanBinary64,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("malformed_source_spellings_never_read"),
        word: Word::Succeeds,
        input: || Scenario::Unreadable(UNREADABLE),
        expect: Expect::Value(19),
    },
    Case {
        name: CaseName("nan_never_forms_an_ordered_pair"),
        word: Word::Succeeds,
        input: || Scenario::NotANumber,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("existing_whole_numbers_do_not_round_through_float"),
        word: Word::Succeeds,
        input: || Scenario::Whole,
        expect: Expect::Value(6),
    },
    Case {
        name: CaseName("integer_successor_preserves_sign_and_precision"),
        word: Word::Succeeds,
        input: || Scenario::Successors(SUCCESSORS),
        expect: Expect::Value(8),
    },
    Case {
        name: CaseName("integer_successor_refuses_floats_and_gaps"),
        word: Word::Succeeds,
        input: || Scenario::NotSuccessors(NOT_SUCCESSORS),
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("integer_successor_crosses_the_whole_number_range"),
        word: Word::Succeeds,
        input: || Scenario::WholeSuccessors,
        expect: Expect::Value(2),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Scenario::Ordered(examples) => ordered(examples),
        Scenario::WiderThanBinary64 => wider_than_binary64(),
        Scenario::Unreadable(texts) => unreadable(texts),
        Scenario::NotANumber => not_a_number(),
        Scenario::Whole => whole(),
        Scenario::Successors(pairs) => successors(pairs),
        Scenario::NotSuccessors(pairs) => not_successors(pairs),
        Scenario::WholeSuccessors => whole_successors(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
