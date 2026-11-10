//! The Ruff annotation fragments (`future-annotation-context`, `annotation-evaluation-context`) against the language's annotation
//! context fixture. The provider's own `compiler-feature` rows are proved in the Python crate's `compiler_feature` item test.
use super::boundary_failure_test::{BoundaryFailure, failure};
use ryo_benchmark::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::path::Path;

use super::authored_queries_test::authored_suppression_cohort_queries_validate_through_public_sdk_contracts;
use super::context_case_test::{ContextCase, check_case, stage_modules};

fn compiler_features_and_annotation_contexts_follow_the_language_contract() -> Result<(), Error> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/compiler-features/annotation-contexts.json");
    let cases: Vec<ContextCase> = serde_json::from_slice(&std::fs::read(source)?)
        .map_err(|error| Error::Json(Box::new(error)))?;
    assert!(
        cases.len() >= 40,
        "positive and negative contexts must remain nonvacuous"
    );
    let temporary = tempfile::tempdir()?;
    let modules = stage_modules(temporary.path())?;
    for case in &cases {
        check_case(temporary.path(), &modules, case)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum CompilerFeatureScenario {
    CompilerFeaturesAndAnnotationContextsFollowTheLanguageContract,
    AuthoredSuppressionCohortQueriesValidateThroughPublicSdkContracts,
}
type Input = CompilerFeatureScenario;
type Value = bool;
type Failure = BoundaryFailure;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("compiler_features_and_annotation_contexts_follow_the_language_contract"),
        word: Word::Succeeds,
        input: || {
            CompilerFeatureScenario::CompilerFeaturesAndAnnotationContextsFollowTheLanguageContract
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("authored_suppression_cohort_queries_validate_through_public_sdk_contracts"),
        word: Word::Succeeds,
        input: || {
            CompilerFeatureScenario::AuthoredSuppressionCohortQueriesValidateThroughPublicSdkContracts
        },
        expect: Expect::Value(true),
    },
];

fn exercise(input: Input) -> Result<Value, Error> {
    match input {
        CompilerFeatureScenario::CompilerFeaturesAndAnnotationContextsFollowTheLanguageContract => compiler_features_and_annotation_contexts_follow_the_language_contract().map(|()| true),
        CompilerFeatureScenario::AuthoredSuppressionCohortQueriesValidateThroughPublicSdkContracts => {
            authored_suppression_cohort_queries_validate_through_public_sdk_contracts();
            Ok(true)
        }
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input).map_err(failure)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
