//! The `constant_numbers` service's cases: the exact key a float normalizes to, its negation, and the floats that become integers.

use super::service::{float_key, negate, to_float};
use super::types::{Binary64, RealKey};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The negation of the key of each float.
    Negated(&'static [f64]),
    /// The key of each float, then the float it converts back to.
    Floats(&'static [f64]),
}
/// What a row is given.
type Input = Given;
/// The keys, in order, joined by `,`.
type Value = &'static str;
/// Nothing is refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("negation_flips_integers_and_floats"),
        word: Word::Succeeds,
        input: || Given::Negated(&[5.0, 2.5, 0.0]),
        expect: Expect::Value("-5,-2.5f,0"),
    },
    Case {
        name: CaseName("whole_floats_key_as_integers_and_the_others_stay_floats"),
        word: Word::Succeeds,
        input: || Given::Floats(&[2.0, 2.5]),
        expect: Expect::Value("2=2.0,2.5f=2.5"),
    },
    Case {
        name: CaseName("floats_past_the_u64_bound_or_infinite_stay_floats"),
        word: Word::Succeeds,
        input: || Given::Floats(&[-0.0, 18_446_744_073_709_551_616.0, f64::INFINITY, -3.0]),
        expect: Expect::Value(
            "0=0.0,1.8446744073709552e19f=1.8446744073709552e19,inff=inf,-3=-3.0",
        ),
    },
];

/// A real key as text: an integer as written, a float with its fraction.
fn shown(key: RealKey) -> String {
    match key {
        RealKey::Integer(value) => value.0.to_string(),
        RealKey::Float(bits) => format!("{:?}f", f64::from_bits(bits.0)),
    }
}

/// Reads the keys the row asks for.
fn subject(given: Input) -> Result<Value, Failure> {
    let texts: Vec<String> = match given {
        Given::Floats(floats) => floats
            .iter()
            .map(|each| {
                let key = float_key(Binary64(*each));
                format!("{}={:?}", shown(key), to_float(key).0)
            })
            .collect(),
        Given::Negated(floats) => floats
            .iter()
            .map(|each| negate(float_key(Binary64(*each))).map_or_else(|| "none".to_owned(), shown))
            .collect(),
    };
    Ok(Box::leak(texts.join(",").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
