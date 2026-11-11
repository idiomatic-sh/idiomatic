//! The `constant_numbers` service's cases: the exact key each parsed numeric or boolean literal normalizes to, its negation, and the
//! floats that become integers.

use super::service::{float_key, negate, number, to_float};
use super::types::{Binary64, RealKey};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::semantic_syntax::service::positions;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The key of every literal of a source, as `real` or `real+imaginary j`.
    Keys(&'static str),
    /// The negation of the real part of every literal of a source.
    Negated(&'static str),
    /// The key of each float, then the float it converts back to.
    Floats(&'static [f64]),
}
/// What a row is given.
type Input = Given;
/// The keys, in source order, joined by `,`.
type Value = &'static str;
/// A source the parser refuses.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("integers_radixes_booleans_and_whole_floats_key_as_integers"),
        word: Word::Succeeds,
        input: || Given::Keys("1\n0x10\nTrue\nFalse\n2.0\n2.5\n"),
        expect: Expect::Value("1+0j,16+0j,1+0j,0+0j,2+0j,2.5f+0j"),
    },
    Case {
        name: CaseName("a_complex_literal_keys_its_imaginary_part"),
        word: Word::Succeeds,
        input: || Given::Keys("3j\n1.5j\n"),
        expect: Expect::Value("0+3j,0+1.5fj"),
    },
    Case {
        name: CaseName("an_integer_past_u64_and_non_numbers_have_no_key"),
        word: Word::Succeeds,
        input: || Given::Keys("18446744073709551616\n'1'\nNone\n"),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("negation_flips_integers_and_floats"),
        word: Word::Succeeds,
        input: || Given::Negated("5\n2.5\n0.0\n"),
        expect: Expect::Value("-5,-2.5f,0"),
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
        Given::Keys(source) | Given::Negated(source) => {
            let file = parsed(source.as_bytes()).map_err(|_| ())?;
            let tree = Tree {
                elements: &file.elements,
                source: ByteSlice(file.bytes),
                file: languages::FileId(0),
            };
            positions(&tree)
                .filter_map(|at| number(&tree, at))
                .map(|key| match given {
                    Given::Negated(_) => negate(key.real).map_or_else(|| "none".to_owned(), shown),
                    Given::Keys(_) | Given::Floats(_) => {
                        format!("{}+{}j", shown(key.real), shown(key.imaginary))
                    }
                })
                .collect()
        }
    };
    Ok(Box::leak(texts.join(",").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
