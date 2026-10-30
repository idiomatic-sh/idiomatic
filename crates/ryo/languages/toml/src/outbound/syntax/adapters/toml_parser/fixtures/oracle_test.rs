//! The oracle's scalar cases: a value the rows say is the library's reading of it, and a `nan` float is not a `nan` string.

use super::{Said, compared, scalar};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What the rows say of a scalar: its kind, its text and what it decodes to.
type Input = (&'static str, &'static str, &'static str);
/// Whether the scalar the rows say equals the library's `nan` float.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_nan_float_is_the_libraries_nan_float"),
        word: Word::Succeeds,
        input: || ("float", "nan", "nan"),
        expect: Expect::Value("equal"),
    },
    Case {
        name: CaseName("a_string_nan_is_not_the_libraries_nan_float"),
        word: Word::Succeeds,
        input: || ("string", "\"nan\"", "nan"),
        expect: Expect::Value("different"),
    },
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "the subject of a case returns a Result for the rows that refuse, whether or not a row of this part's cases does"
)]
fn subject((kind, text, decoded): Input) -> Result<Value, Failure> {
    let said = Said {
        ordinal: 0,
        key: "v".to_owned(),
        kind: kind.to_owned(),
        at: 0,
        text: text.to_owned(),
        decoded: decoded.to_owned(),
    };
    Ok(
        if scalar(&said) == Ok(compared(toml::Value::Float(f64::NAN))) {
            "equal"
        } else {
            "different"
        },
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
