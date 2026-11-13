//! The `decoded` service's cases, on values alone.

use super::constants::FLOAT_TEXT_BYTES;
use super::service::{binary64_text, decoded_at};
use super::types::{FloatBits, FloatBuffer};
use crate::domain::builders::service::{
    decoded_token, enter_node, leave_node, table_builder, token,
};
use crate::domain::elements::types::Elements;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{KindBits, Offset, Position};
use ryo_relations::domain::terms::types::NameText;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// A scenario: what a row asks of the service, observed as text.
type Input = fn() -> String;
/// What a scenario observed, each check in order, joined by `|`.
type Value = &'static str;
type Failure = ();

fn a_decoded_token_keeps_the_text_its_library_read_and_a_plain_one_has_none() -> String {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"\"\\u{41}\"fn"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    decoded_token(
        &mut builder,
        KindBits(2),
        Offset(0),
        Offset(8),
        NameText("A"),
    );
    token(&mut builder, KindBits(3), Offset(8), Offset(10));
    leave_node(&mut builder, Offset(10));
    format!(
        "{:?}|{:?}|{:?}",
        decoded_at(&elements, Position(1)),
        decoded_at(&elements, Position(2)),
        decoded_at(&elements, Position(0)),
    )
}

/// The spelling of the value a decimal text parses to, owned.
fn spelled(written: &str) -> String {
    let value = written
        .parse::<f64>()
        .expect("the row writes a binary64 value");
    let mut buffer = FloatBuffer([0; FLOAT_TEXT_BYTES]);
    binary64_text(FloatBits(value.to_bits()), &mut buffer)
        .0
        .to_owned()
}

fn a_float_is_spelled_once_as_every_language_writes_it() -> String {
    ["1", "1e21", "0.1", "-0", "inf", "NaN"]
        .map(spelled)
        .join("|")
}

fn the_widest_whole_float_fits_without_an_exponent() -> String {
    let text = spelled("1e300");
    format!(
        "{}|{}|{}",
        text.len(),
        text.trim_end_matches(".0")
            .trim_start_matches('1')
            .trim_matches('0')
            .is_empty(),
        text.ends_with(".0")
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_decoded_token_keeps_the_text_its_library_read_and_a_plain_one_has_none"),
        word: Word::Succeeds,
        input: || a_decoded_token_keeps_the_text_its_library_read_and_a_plain_one_has_none,
        expect: Expect::Value("Some(NameText(\"A\"))|None|None"),
    },
    Case {
        name: CaseName("a_float_is_spelled_once_as_every_language_writes_it"),
        word: Word::Succeeds,
        input: || a_float_is_spelled_once_as_every_language_writes_it,
        expect: Expect::Value("1.0|1000000000000000000000.0|0.1|-0.0|inf|NaN"),
    },
    Case {
        name: CaseName("the_widest_whole_float_fits_without_an_exponent"),
        word: Word::Succeeds,
        input: || the_widest_whole_float_fits_without_an_exponent,
        expect: Expect::Value("303|true|true"),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Box::leak(scenario().into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
