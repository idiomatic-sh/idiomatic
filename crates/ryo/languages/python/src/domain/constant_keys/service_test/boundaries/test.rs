//! Independent boundary checks for typed constant keys.

use super::fixtures::{
    float_boundaries_and_huge_integer_structural_keys_are_retained,
    invalidation_clears_occurrences_but_keeps_the_reader_reusable,
    supported_numeric_keys_normalize_and_unsupported_expressions_are_absent,
    underflow_subnormal_infinity_complex_and_unary_values_follow_typed_payloads,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `supported_numeric_keys_normalize_and_unsupported_expressions_are_absent` scenario.
    SupportedNumericKeysNormalizeAndUnsupportedExpressionsAreAbsent,
    /// The `float_boundaries_and_huge_integer_structural_keys_are_retained` scenario.
    FloatBoundariesAndHugeIntegerStructuralKeysAreRetained,
    /// The `underflow_subnormal_infinity_complex_and_unary_values_follow_typed_payloads` scenario.
    UnderflowSubnormalInfinityComplexAndUnaryValuesFollowTypedPayloads,
    /// The `invalidation_clears_occurrences_but_keeps_the_reader_reusable` scenario.
    InvalidationClearsOccurrencesButKeepsTheReaderReusable,
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
        name: CaseName("supported_numeric_keys_normalize_and_unsupported_expressions_are_absent"),
        word: Word::Succeeds,
        input: || Given::SupportedNumericKeysNormalizeAndUnsupportedExpressionsAreAbsent,
        expect: Expect::Value(
            "Complete|true|true|true|true|true|true|true|true|true|true|true|true|true",
        ),
    },
    Case {
        name: CaseName("float_boundaries_and_huge_integer_structural_keys_are_retained"),
        word: Word::Succeeds,
        input: || Given::FloatBoundariesAndHugeIntegerStructuralKeysAreRetained,
        expect: Expect::Value("[]|Some(Position(14))|true|2|Some(Position(22))|Some(Position(32))"),
    },
    Case {
        name: CaseName(
            "underflow_subnormal_infinity_complex_and_unary_values_follow_typed_payloads",
        ),
        word: Word::Succeeds,
        input: || Given::UnderflowSubnormalInfinityComplexAndUnaryValuesFollowTypedPayloads,
        expect: Expect::Value(
            "Some(Position(76))|true|true|Some(Position(76))|true|Some(Position(34))|Some(Position(50))|Some(Position(66))|Some(Position(76))|true",
        ),
    },
    Case {
        name: CaseName("invalidation_clears_occurrences_but_keeps_the_reader_reusable"),
        word: Word::Succeeds,
        input: || Given::InvalidationClearsOccurrencesButKeepsTheReaderReusable,
        expect: Expect::Value("true|true|true|true"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::SupportedNumericKeysNormalizeAndUnsupportedExpressionsAreAbsent => {
            supported_numeric_keys_normalize_and_unsupported_expressions_are_absent()
        }
        Given::FloatBoundariesAndHugeIntegerStructuralKeysAreRetained => {
            float_boundaries_and_huge_integer_structural_keys_are_retained()
        }
        Given::UnderflowSubnormalInfinityComplexAndUnaryValuesFollowTypedPayloads => {
            underflow_subnormal_infinity_complex_and_unary_values_follow_typed_payloads()
        }
        Given::InvalidationClearsOccurrencesButKeepsTheReaderReusable => {
            invalidation_clears_occurrences_but_keeps_the_reader_reusable()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
