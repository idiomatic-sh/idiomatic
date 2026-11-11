//! The observation describes the standard mini-language, not a custom value's `__format__`.

use super::fixtures::{
    dynamic_outer_is_deferred_but_nested_specs_can_be_observed_separately,
    static_clauses_and_unicode_fill_are_recognized,
    unsupported_type_is_distinct_from_other_structural_error, zero_coercion_is_version_bound,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `static_clauses_and_unicode_fill_are_recognized` scenario.
    StaticClausesAndUnicodeFillAreRecognized,
    /// The `unsupported_type_is_distinct_from_other_structural_error` scenario.
    UnsupportedTypeIsDistinctFromOtherStructuralError,
    /// The `dynamic_outer_is_deferred_but_nested_specs_can_be_observed_separately` scenario.
    DynamicOuterIsDeferredButNestedSpecsCanBeObservedSeparately,
    /// The `zero_coercion_is_version_bound` scenario.
    ZeroCoercionIsVersionBound,
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
        name: CaseName("static_clauses_and_unicode_fill_are_recognized"),
        word: Word::Succeeds,
        input: || Given::StaticClausesAndUnicodeFillAreRecognized,
        expect: Expect::Value(
            "Recognized|Recognized|Recognized|Recognized|Recognized|Recognized|Recognized|Recognized|Recognized",
        ),
    },
    Case {
        name: CaseName("unsupported_type_is_distinct_from_other_structural_error"),
        word: Word::Succeeds,
        input: || Given::UnsupportedTypeIsDistinctFromOtherStructuralError,
        expect: Expect::Value(
            "InvalidType(PresentationCharacter('z'))|InvalidType(PresentationCharacter('y'))|InvalidType(PresentationCharacter('}'))|OtherError|OtherError|OtherError|InvalidType(PresentationCharacter('q'))|InvalidType(PresentationCharacter('²'))",
        ),
    },
    Case {
        name: CaseName("dynamic_outer_is_deferred_but_nested_specs_can_be_observed_separately"),
        word: Word::Succeeds,
        input: || Given::DynamicOuterIsDeferredButNestedSpecsCanBeObservedSeparately,
        expect: Expect::Value("Dynamic|InvalidType(PresentationCharacter('y'))"),
    },
    Case {
        name: CaseName("zero_coercion_is_version_bound"),
        word: Word::Succeeds,
        input: || Given::ZeroCoercionIsVersionBound,
        expect: Expect::Value("InvalidType(PresentationCharacter('z'))|Recognized"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::StaticClausesAndUnicodeFillAreRecognized => {
            static_clauses_and_unicode_fill_are_recognized()
        }
        Given::UnsupportedTypeIsDistinctFromOtherStructuralError => {
            unsupported_type_is_distinct_from_other_structural_error()
        }
        Given::DynamicOuterIsDeferredButNestedSpecsCanBeObservedSeparately => {
            dynamic_outer_is_deferred_but_nested_specs_can_be_observed_separately()
        }
        Given::ZeroCoercionIsVersionBound => zero_coercion_is_version_bound(),
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
