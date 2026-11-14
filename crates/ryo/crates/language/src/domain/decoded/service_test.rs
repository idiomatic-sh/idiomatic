//! The `decoded` service's cases, on values alone.

use super::constants::FLOAT_TEXT_BYTES;
use super::service::{binary64_text, decoded_at, exact_digits};
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

/// The bits a literal's source text decodes to: a prefixed integer in its base, rounded to the nearest binary64, or a decimal.
fn decoded(literal: &str) -> FloatBits {
    let literal = literal.replace('_', "");
    let radix = match literal.get(..2) {
        Some("0x" | "0X") => 16,
        Some("0o" | "0O") => 8,
        Some("0b" | "0B") => 2,
        _ => 10,
    };
    let value = if radix == 10 {
        literal.parse::<f64>().expect("the row writes a decimal literal")
    } else {
        #[expect(
            clippy::cast_precision_loss,
            reason = "the literal's integer rounds to the nearest binary64, as a parser decodes it"
        )]
        let rounded = u128::from_str_radix(&literal[2..], radix).expect("the row writes a prefixed integer") as f64;
        rounded
    };
    FloatBits(value.to_bits())
}

/// Each row's `exact_digits`, for the digits a provider writes (separators taken out) and the literal it decoded.
fn exactness(rows: &[(&str, &str)]) -> String {
    rows.iter()
        .map(|(written, literal)| {
            format!("{:?}", exact_digits(NameText(&written.replace('_', "")), decoded(literal)))
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// oxlint's `no-loss-of-precision` passes: decimal digits a binary64 gives back.
fn decimal_digits_a_binary64_gives_back_are_exact() -> String {
    exactness(&[
        ("12345", "12345"),
        ("123.456", "123.456"),
        ("12300000000000000000000000", "12300000000000000000000000"),
        ("0.00000000000000000000000123", "0.00000000000000000000000123"),
        ("9007199254740991", "9007199254740991"),
        ("0", "0"),
        ("0.0", "0.0"),
        ("123.0000000000000000000000", "123.0000000000000000000000"),
        ("9.00", "9.00e2"),
        ("9.100", "9.100E3"),
        ("019.5", "019.5"),
        ("0008", "0008"),
        ("0", "0e5"),
        (".42", ".42"),
        ("42.", "42."),
        ("123.0", "123.0e34"),
        ("123_00_000_000_000_000_000_000_000", "123_00_000_000_000_000_000_000_000"),
        ("123.000_000_000_000_000_000_000_0", "123.000_000_000_000_000_000_000_0"),
        ("480.00", "480.00"),
        ("3", "3e-308"),
        ("1000000000000000128", "1000000000000000128"),
    ])
}

/// oxlint's `no-loss-of-precision` failures: decimal digits a binary64 does not give back.
fn decimal_digits_a_binary64_rounds_are_not_exact() -> String {
    exactness(&[
        ("9007199254740993", "9007199254740993"),
        ("9007199254740.993", "9007199254740.993e3"),
        ("9.007199254740993", "9.007199254740993e15"),
        ("900719.9254740994", "900719.9254740994"),
        (".9007199254740993", ".9007199254740993e16"),
        ("900719925474099.30", "900719925474099.30e1"),
        ("90071992547409930", "90071992547409930e-1"),
        ("5123000000000000000000000000001", "5123000000000000000000000000001"),
        ("1230000000000000000000000.0", "1230000000000000000000000.0"),
        ("1.0000000000000000000000123", "1.0000000000000000000000123"),
        (
            "17498005798264095394980017816940970922825355447145699491406164851279623993595007385788105416184430592",
            "17498005798264095394980017816940970922825355447145699491406164851279623993595007385788105416184430592",
        ),
        ("2", "2e999"),
        (".1230000000000000000000000", ".1230000000000000000000000"),
        ("1", "1e18446744073709551615"),
        ("96215808661.52751", "96215808661.52751e-84"),
        ("1", "1e-400"),
    ])
}

/// oxlint's `no-loss-of-precision` passes (its cases at 662-682): prefixed integers of at most 53 significant bits.
fn prefixed_digits_a_binary64_holds_are_exact() -> String {
    exactness(&[
        ("0b11111111111111111111111111111111111111111111111111111", "0b11111111111111111111111111111111111111111111111111111"),
        ("0b111_111_111_111_1111_11111_111_11111_1111111111_11111111_111_111", "0b111_111_111_111_1111_11111_111_11111_1111111111_11111111_111_111"),
        ("0o377777777777777777", "0o377777777777777777"),
        ("0o3_77_777_777_777_777_777", "0o3_77_777_777_777_777_777"),
        ("0x1FFFFFFFFFFFFF", "0x1FFFFFFFFFFFFF"),
        ("0x1fffffffffffff", "0x1fffffffffffff"),
        ("0x1FFF_FFFF_FFF_FFF", "0x1FFF_FFFF_FFF_FFF"),
        ("0x1_FFF_FFFF_FFF_FFF", "0x1_FFF_FFFF_FFF_FFF"),
        ("0x0", "0x0"),
        ("0x20000000000000", "0x20000000000000"),
    ])
}

/// oxlint's `no-loss-of-precision` failures (its cases at 733-756): prefixed integers binary64 rounds.
fn prefixed_digits_a_binary64_rounds_are_not_exact() -> String {
    exactness(&[
        ("0b100000000000000000000000000000000000000000000000000001", "0b100000000000000000000000000000000000000000000000000001"),
        ("0o400000000000000001", "0o400000000000000001"),
        ("0x20000000000001", "0x20000000000001"),
        ("0b1_0000000000000000000000000000000000000000000000000000_1", "0b1_0000000000000000000000000000000000000000000000000000_1"),
        ("0b10000000000_0000000000000000000000000000_000000000000001", "0b10000000000_0000000000000000000000000000_000000000000001"),
        ("0o4_00000000000000_001", "0o4_00000000000000_001"),
        ("0o4_0000000000000000_1", "0o4_0000000000000000_1"),
        ("0x2_0000000000001", "0x2_0000000000001"),
        ("0x200000_0000000_1", "0x200000_0000000_1"),
        ("0b100_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_001", "0b100_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_001"),
    ])
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
    Case {
        name: CaseName("decimal_digits_a_binary64_gives_back_are_exact"),
        word: Word::Succeeds,
        input: || decimal_digits_a_binary64_gives_back_are_exact,
        expect: Expect::Value("Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised"),
    },
    Case {
        name: CaseName("decimal_digits_a_binary64_rounds_are_not_exact"),
        word: Word::Succeeds,
        input: || decimal_digits_a_binary64_rounds_are_not_exact,
        expect: Expect::Value("Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered"),
    },
    Case {
        name: CaseName("prefixed_digits_a_binary64_holds_are_exact"),
        word: Word::Succeeds,
        input: || prefixed_digits_a_binary64_holds_are_exact,
        expect: Expect::Value("Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised|Raised"),
    },
    Case {
        name: CaseName("prefixed_digits_a_binary64_rounds_are_not_exact"),
        word: Word::Succeeds,
        input: || prefixed_digits_a_binary64_rounds_are_not_exact,
        expect: Expect::Value("Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered|Lowered"),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Box::leak(scenario().into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
