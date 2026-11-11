//! Request identity: corpus substitution and per-run generation.
use super::super::request_identity::service::RequestIdentity;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::files::types::InputReadFailure;
use std::sync::Arc;
const EDGE_REFUSAL: &str = "actual edge refusal";
const FIRST: &str = "first";
const SECOND: &str = "second";
const SAME: &str = "same";
fn corpus(path: FilePath) -> Vec<CorpusInput> {
    vec![CorpusInput::Unreadable {
        path,
        reason: InputReadFailure(EDGE_REFUSAL.into()),
    }]
}
fn equal_inventory_size_does_not_authorize_corpus_substitution() {
    let first = corpus(FilePath(FIRST.into()));
    let identity = RequestIdentity::new(&first);
    assert_eq!(identity.corpus(), first.as_slice());
    assert_ne!(
        identity.corpus(),
        corpus(FilePath(SECOND.into())).as_slice()
    );
}
fn each_run_has_a_distinct_generation_even_with_identical_metadata() {
    let corpus = corpus(FilePath(SAME.into()));
    let first = Arc::new(RequestIdentity::new(&corpus));
    let same = Arc::clone(&first);
    let later = Arc::new(RequestIdentity::new(&corpus));
    assert!(first == same);
    assert!(first != later);
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    EqualInventorySizeDoesNotAuthorizeCorpusSubstitution,
    EachRunHasADistinctGenerationEvenWithIdenticalMetadata,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_inventory_size_does_not_authorize_corpus_substitution"),
        word: Word::Succeeds,
        input: || Scenario::EqualInventorySizeDoesNotAuthorizeCorpusSubstitution,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("each_run_has_a_distinct_generation_even_with_identical_metadata"),
        word: Word::Succeeds,
        input: || Scenario::EachRunHasADistinctGenerationEvenWithIdenticalMetadata,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::EqualInventorySizeDoesNotAuthorizeCorpusSubstitution => {
            equal_inventory_size_does_not_authorize_corpus_substitution();
            Ok(())
        }
        Scenario::EachRunHasADistinctGenerationEvenWithIdenticalMetadata => {
            each_run_has_a_distinct_generation_even_with_identical_metadata();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
