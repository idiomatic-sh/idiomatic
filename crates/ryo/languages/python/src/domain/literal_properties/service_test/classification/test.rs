//! Parser-owned literal values classify without inferring runtime truthiness.

use super::fixtures::{
    boolean_parser_payload_is_written_in_optimized_builds,
    f_string_fragments_are_excluded_but_interpolation_expressions_are_literals,
    primitive_values_use_parser_payload_and_decoded_tokens,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `primitive_values_use_parser_payload_and_decoded_tokens` scenario.
    PrimitiveValuesUseParserPayloadAndDecodedTokens,
    /// The `boolean_parser_payload_is_written_in_optimized_builds` scenario.
    BooleanParserPayloadIsWrittenInOptimizedBuilds,
    /// The `f_string_fragments_are_excluded_but_interpolation_expressions_are_literals` scenario.
    FStringFragmentsAreExcludedButInterpolationExpressionsAreLiterals,
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
        name: CaseName("primitive_values_use_parser_payload_and_decoded_tokens"),
        word: Word::Succeeds,
        input: || Given::PrimitiveValuesUseParserPayloadAndDecodedTokens,
        expect: Expect::Value(
            "[Properties { kind: NameText(\"integer\"), zero: Raised, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"integer\"), zero: Raised, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"integer\"), zero: Raised, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"integer\"), zero: Raised, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"integer\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"integer\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"float\"), zero: Raised, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"float\"), zero: Raised, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"float\"), zero: Raised, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"float\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"float\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"complex\"), zero: Raised, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"complex\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"boolean\"), zero: Lowered, empty: Lowered, boolean: Raised }, Properties { kind: NameText(\"boolean\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"null\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"ellipsis\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"bytes\"), zero: Lowered, empty: Raised, boolean: Lowered }, Properties { kind: NameText(\"bytes\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"bytes\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"string\"), zero: Lowered, empty: Raised, boolean: Lowered }, Properties { kind: NameText(\"string\"), zero: Lowered, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"string\"), zero: Lowered, empty: Lowered, boolean: Lowered }]",
        ),
    },
    Case {
        name: CaseName("boolean_parser_payload_is_written_in_optimized_builds"),
        word: Word::Succeeds,
        input: || Given::BooleanParserPayloadIsWrittenInOptimizedBuilds,
        expect: Expect::Value(
            "[Properties { kind: NameText(\"boolean\"), zero: Lowered, empty: Lowered, boolean: Raised }]",
        ),
    },
    Case {
        name: CaseName(
            "f_string_fragments_are_excluded_but_interpolation_expressions_are_literals",
        ),
        word: Word::Succeeds,
        input: || Given::FStringFragmentsAreExcludedButInterpolationExpressionsAreLiterals,
        expect: Expect::Value(
            "[Properties { kind: NameText(\"integer\"), zero: Raised, empty: Lowered, boolean: Lowered }, Properties { kind: NameText(\"string\"), zero: Lowered, empty: Raised, boolean: Lowered }, Properties { kind: NameText(\"integer\"), zero: Lowered, empty: Lowered, boolean: Lowered }]",
        ),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::PrimitiveValuesUseParserPayloadAndDecodedTokens => {
            primitive_values_use_parser_payload_and_decoded_tokens()
        }
        Given::BooleanParserPayloadIsWrittenInOptimizedBuilds => {
            boolean_parser_payload_is_written_in_optimized_builds()
        }
        Given::FStringFragmentsAreExcludedButInterpolationExpressionsAreLiterals => {
            f_string_fragments_are_excluded_but_interpolation_expressions_are_literals()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
