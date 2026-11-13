//! Parser values normalize every Python radix without evaluating unary or arithmetic expressions.

use super::fixtures::{
    floats_round_trip_and_keep_their_numeric_kind,
    integer_payloads_are_canonical_arbitrary_precision_and_exclude_other_primitives,
    literals_inside_interpolations_remain_real_numeric_occurrences,
    unary_signs_and_arithmetic_remain_separate_syntax,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `integer_payloads_are_canonical_arbitrary_precision_and_exclude_other_primitives` scenario.
    IntegerPayloadsAreCanonicalArbitraryPrecisionAndExcludeOtherPrimitives,
    /// The `unary_signs_and_arithmetic_remain_separate_syntax` scenario.
    UnarySignsAndArithmeticRemainSeparateSyntax,
    /// The `floats_round_trip_and_keep_their_numeric_kind` scenario.
    FloatsRoundTripAndKeepTheirNumericKind,
    /// The `literals_inside_interpolations_remain_real_numeric_occurrences` scenario.
    LiteralsInsideInterpolationsRemainRealNumericOccurrences,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "integer_payloads_are_canonical_arbitrary_precision_and_exclude_other_primitives",
        ),
        word: Word::Succeeds,
        input: || Given::IntegerPayloadsAreCanonicalArbitraryPrecisionAndExcludeOtherPrimitives,
        expect: Expect::Value(
            "[(\"integer\", \"16\"), (\"integer\", \"16\"), (\"integer\", \"16\"), (\"integer\", \"1000\"), (\"integer\", \"9007199254740993\"), (\"integer\", \"18446744073709551616\"), (\"integer\", \"340282366920938463463374607431768211457\")]",
        ),
    },
    Case {
        name: CaseName("unary_signs_and_arithmetic_remain_separate_syntax"),
        word: Word::Succeeds,
        input: || Given::UnarySignsAndArithmeticRemainSeparateSyntax,
        expect: Expect::Value("[\"42\", \"16\", \"1\", \"2\", \"0.0\"]"),
    },
    Case {
        name: CaseName("floats_round_trip_and_keep_their_numeric_kind"),
        word: Word::Succeeds,
        input: || Given::FloatsRoundTripAndKeepTheirNumericKind,
        expect: Expect::Value(
            "\"float\"|Ok(4607182418800017408)|true|\"float\"|Ok(4652007308841189376)|true|\"float\"|Ok(4652007308841189376)|true|\"float\"|Ok(1)|true|\"float\"|Ok(0)|true|\"float\"|Ok(9218868437227405312)|true|6|\"-0.0\"|\"-inf\"|\"NaN\"|true|Ok(1)|true|Ok(9223372036854775809)|true|Ok(9218868437227405311)|true|Ok(18442240474082181119)",
        ),
    },
    Case {
        name: CaseName("literals_inside_interpolations_remain_real_numeric_occurrences"),
        word: Word::Succeeds,
        input: || Given::LiteralsInsideInterpolationsRemainRealNumericOccurrences,
        expect: Expect::Value("[\"16\", \"1.0\"]"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::IntegerPayloadsAreCanonicalArbitraryPrecisionAndExcludeOtherPrimitives => {
            integer_payloads_are_canonical_arbitrary_precision_and_exclude_other_primitives()
        }
        Given::UnarySignsAndArithmeticRemainSeparateSyntax => {
            unary_signs_and_arithmetic_remain_separate_syntax()
        }
        Given::FloatsRoundTripAndKeepTheirNumericKind => {
            floats_round_trip_and_keep_their_numeric_kind()
        }
        Given::LiteralsInsideInterpolationsRemainRealNumericOccurrences => {
            literals_inside_interpolations_remain_real_numeric_occurrences()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
