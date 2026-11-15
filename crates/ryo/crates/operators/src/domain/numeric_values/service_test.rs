//! Precision boundaries are specified independently of the implementation's representation.

use super::{
    service::{compare, integer_successor, read},
    types::{FloatValue, NumberText, Numeric, Succession},
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::WholeNumber;
use std::borrow::Cow;
use std::cmp::Ordering::{self, Equal, Greater, Less};

/// A numeric operand: text read as a number, a whole number, a float, or the integer two to this power, as text.
#[derive(Clone, Copy, Debug)]
enum Num {
    Text(&'static str),
    Whole(i64),
    Float(f64),
    TwoToThe(u32),
}

/// What a row asks of the part.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The order of each pair, both ways.
    Compare(&'static [(Num, Num)]),
    /// Whether each text reads as normalized numeric text.
    Read(&'static [&'static str]),
    /// Whether, of each pair, the right succeeds the left, the left the right, and the left itself.
    Successor(&'static [(Num, Num)]),
}

/// What the part answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Orders(Cow<'static, [(Option<Ordering>, Option<Ordering>)]>),
    Read(Cow<'static, [bool]>),
    Successions(Cow<'static, [[Succession; 3]]>),
}

type Input = Ask;
type Value = Answer;
/// Nothing in this part refuses.
type Failure = std::convert::Infallible;

const ORDERED: &[(Num, Num)] = &[
    (Num::Text("0"), Num::Text("-0")),
    (Num::Text("0"), Num::Text("-0.0")),
    (Num::Text("1"), Num::Text("1.0")),
    (Num::Text("-2"), Num::Text("-1")),
    (
        Num::Text("100000000000000000000"),
        Num::Text("99999999999999999999"),
    ),
    (Num::Text("9007199254740993"), Num::Text("9007199254740992")),
    (
        Num::Text("9007199254740993"),
        Num::Text("9007199254740992.0"),
    ),
    (
        Num::Text("-9007199254740993"),
        Num::Text("-9007199254740992.0"),
    ),
    (
        Num::Text("18446744073709551615"),
        Num::Text("1.8446744073709552e19"),
    ),
    (
        Num::Text("18446744073709551617"),
        Num::Text("1.8446744073709552e19"),
    ),
    (
        Num::Text("340282366920938463463374607431768211455"),
        Num::Text("3.402823669209385e38"),
    ),
    (
        Num::Text("340282366920938463463374607431768211456"),
        Num::Text("3.402823669209385e38"),
    ),
    (
        Num::Text("340282366920938463463374607431768211457"),
        Num::Text("3.402823669209385e38"),
    ),
    (
        Num::Text("-340282366920938463463374607431768211457"),
        Num::Text("-3.402823669209385e38"),
    ),
    (
        Num::Text("340282366920938463463374607431768211456"),
        Num::Text("1.0"),
    ),
    (Num::Text("0"), Num::Text("5e-324")),
    (Num::Text("1"), Num::Text("5e-324")),
    (Num::Text("-1"), Num::Text("-5e-324")),
    (Num::Text("0"), Num::Text("2.2250738585072014e-308")),
    (Num::Text("1"), Num::Text("0.9999999999999999")),
    (Num::Text("1"), Num::Text("1.0000000000000002")),
    (Num::Text("86400.25"), Num::Text("86400")),
    (Num::Text("86400.25"), Num::Text("86401")),
    (Num::Text("inf"), Num::Text("1.7976931348623157e308")),
    (
        Num::Text("-inf"),
        Num::Text("-99999999999999999999999999999999999"),
    ),
    (Num::Text("inf"), Num::Text("inf")),
    (Num::Text("-inf"), Num::Text("-inf")),
];

/// The order of each pair of `ORDERED`, and of its reverse.
const ORDERS: &[(Option<Ordering>, Option<Ordering>)] = &[
    (Some(Equal), Some(Equal)),
    (Some(Equal), Some(Equal)),
    (Some(Equal), Some(Equal)),
    (Some(Less), Some(Greater)),
    (Some(Greater), Some(Less)),
    (Some(Greater), Some(Less)),
    (Some(Greater), Some(Less)),
    (Some(Less), Some(Greater)),
    (Some(Less), Some(Greater)),
    (Some(Greater), Some(Less)),
    (Some(Less), Some(Greater)),
    (Some(Equal), Some(Equal)),
    (Some(Greater), Some(Less)),
    (Some(Less), Some(Greater)),
    (Some(Greater), Some(Less)),
    (Some(Less), Some(Greater)),
    (Some(Greater), Some(Less)),
    (Some(Less), Some(Greater)),
    (Some(Less), Some(Greater)),
    (Some(Greater), Some(Less)),
    (Some(Less), Some(Greater)),
    (Some(Greater), Some(Less)),
    (Some(Less), Some(Greater)),
    (Some(Greater), Some(Less)),
    (Some(Less), Some(Greater)),
    (Some(Equal), Some(Equal)),
    (Some(Equal), Some(Equal)),
];

const UNREADABLE: &[&str] = &[
    "", " 1", "1 ", "+1", "01", "1_000", "0xff", "0o10", "0b10", "1j", "True", "--1", "1.", ".5",
    "1e", "1e+", "1.0x", "00.0", "Infinity",
];

const SUCCESSORS: &[(Num, Num)] = &[
    (Num::Text("-2"), Num::Text("-1")),
    (Num::Text("-1"), Num::Text("0")),
    (Num::Text("-0"), Num::Text("1")),
    (Num::Text("42"), Num::Text("43")),
    (Num::Text("9007199254740992"), Num::Text("9007199254740993")),
    (
        Num::Text("9223372036854775807"),
        Num::Text("9223372036854775808"),
    ),
    (
        Num::Text("170141183460469231731687303715884105727"),
        Num::Text("170141183460469231731687303715884105728"),
    ),
    (
        Num::Text("-170141183460469231731687303715884105728"),
        Num::Text("-170141183460469231731687303715884105727"),
    ),
];

const NOT_SUCCESSORS: &[(Num, Num)] = &[
    (Num::Text("1.0"), Num::Text("2")),
    (Num::Text("1"), Num::Text("2.0")),
    (Num::Text("inf"), Num::Text("2")),
    (Num::Text("1"), Num::Text("NaN")),
    (Num::Text("42"), Num::Text("44")),
];

/// The text of a numeric operand, written out for the integer two to a power; none for a number given as such.
fn spelled(num: Num) -> Cow<'static, str> {
    match num {
        Num::Text(text) => Cow::Borrowed(text),
        Num::TwoToThe(power) => Cow::Owned((num_bigint::BigUint::from(1_u8) << power).to_string()),
        Num::Whole(_) | Num::Float(_) => Cow::Borrowed(""),
    }
}

/// The numeric value of an operand, its text read from `spelling`.
fn numeric(num: Num, spelling: &str) -> Numeric<'_> {
    match num {
        Num::Text(_) | Num::TwoToThe(_) => {
            read(NumberText(spelling)).expect("canonical numeric example")
        }
        Num::Whole(whole) => Numeric::Whole(WholeNumber(whole)),
        Num::Float(float) => Numeric::Float(FloatValue(float)),
    }
}

/// The answer of `each` to every pair, each operand spelled where it must be.
fn pairs<T>(pairs: &[(Num, Num)], each: impl Fn(Numeric<'_>, Numeric<'_>) -> T) -> Vec<T> {
    pairs
        .iter()
        .map(|(left, right)| {
            let (left_text, right_text) = (spelled(*left), spelled(*right));
            each(numeric(*left, &left_text), numeric(*right, &right_text))
        })
        .collect()
}

fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(match ask {
        Ask::Compare(examples) => Answer::Orders(
            pairs(examples, |left, right| {
                (compare(left, right), compare(right, left))
            })
            .into(),
        ),
        Ask::Read(texts) => Answer::Read(
            texts
                .iter()
                .map(|text| read(NumberText(text)).is_some())
                .collect(),
        ),
        Ask::Successor(examples) => Answer::Successions(
            pairs(examples, |left, right| {
                [
                    integer_successor(left, right),
                    integer_successor(right, left),
                    integer_successor(left, left),
                ]
            })
            .into(),
        ),
    })
}

const NAN: Num = Num::Text("NaN");

/// Each pair of `NOT_A_NUMBER`, unordered both ways.
const UNORDERED: &[(Option<Ordering>, Option<Ordering>)] = &[(None, None); 3];

const WHOLE: &[(Num, Num)] = &[
    (Num::Whole(i64::MAX), Num::Text("9223372036854775807")),
    (Num::Whole(i64::MAX), Num::Text("9223372036854775808")),
    (Num::Whole(i64::MAX), Num::Text("9223372036854775808.0")),
    (Num::Whole(i64::MIN), Num::Text("-9223372036854775808")),
    (Num::Whole(-1), Num::Text("-1.0")),
    (Num::Whole(0), Num::Text("-1")),
];

const WHOLE_ORDERS: &[(Option<Ordering>, Option<Ordering>)] = &[
    (Some(Equal), Some(Equal)),
    (Some(Less), Some(Greater)),
    (Some(Less), Some(Greater)),
    (Some(Equal), Some(Equal)),
    (Some(Equal), Some(Equal)),
    (Some(Greater), Some(Less)),
];

/// The right succeeds the left; neither the left the right nor the left itself.
const SUCCEEDS: [Succession; 3] = [
    Succession::Successor,
    Succession::NotSuccessor,
    Succession::NotSuccessor,
];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("integer_float_and_wide_mixed_ordering_is_exact"),
        word: Word::Succeeds,
        input: || Ask::Compare(ORDERED),
        expect: Expect::Value(Answer::Orders(Cow::Borrowed(ORDERS))),
    },
    Case {
        name: CaseName("a_wider_integer_than_binary64_orders_against_its_extremes"),
        word: Word::Succeeds,
        input: || {
            Ask::Compare(&[
                (Num::TwoToThe(1024), Num::Float(f64::MAX)),
                (Num::TwoToThe(1024), Num::Float(f64::INFINITY)),
            ])
        },
        expect: Expect::Value(Answer::Orders(Cow::Borrowed(&[
            (Some(Greater), Some(Less)),
            (Some(Less), Some(Greater)),
        ]))),
    },
    Case {
        name: CaseName("malformed_source_spellings_never_read"),
        word: Word::Succeeds,
        input: || Ask::Read(UNREADABLE),
        expect: Expect::Value(Answer::Read(Cow::Borrowed(&[false; 19]))),
    },
    Case {
        name: CaseName("nan_never_forms_an_ordered_pair"),
        word: Word::Succeeds,
        input: || {
            Ask::Compare(&[
                (NAN, Num::Whole(0)),
                (NAN, Num::Float(f64::INFINITY)),
                (NAN, NAN),
            ])
        },
        expect: Expect::Value(Answer::Orders(Cow::Borrowed(UNORDERED))),
    },
    Case {
        name: CaseName("existing_whole_numbers_do_not_round_through_float"),
        word: Word::Succeeds,
        input: || Ask::Compare(WHOLE),
        expect: Expect::Value(Answer::Orders(Cow::Borrowed(WHOLE_ORDERS))),
    },
    Case {
        name: CaseName("integer_successor_preserves_sign_and_precision"),
        word: Word::Succeeds,
        input: || Ask::Successor(SUCCESSORS),
        expect: Expect::Value(Answer::Successions(Cow::Borrowed(&[SUCCEEDS; 8]))),
    },
    Case {
        name: CaseName("integer_successor_refuses_floats_and_gaps"),
        word: Word::Succeeds,
        input: || Ask::Successor(NOT_SUCCESSORS),
        expect: Expect::Value(Answer::Successions(Cow::Borrowed(&[
            [Succession::NotSuccessor; 3],
            [Succession::NotSuccessor; 3],
            [Succession::NotSuccessor; 3],
            [Succession::NotSuccessor; 3],
            [Succession::NotSuccessor; 3],
        ]))),
    },
    Case {
        name: CaseName("integer_successor_crosses_the_whole_number_range"),
        word: Word::Succeeds,
        input: || {
            Ask::Successor(&[
                (Num::Whole(i64::MAX), Num::Text("9223372036854775808")),
                (Num::Whole(i64::MIN), Num::Text("-9223372036854775807")),
            ])
        },
        expect: Expect::Value(Answer::Successions(Cow::Borrowed(&[SUCCEEDS; 2]))),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
