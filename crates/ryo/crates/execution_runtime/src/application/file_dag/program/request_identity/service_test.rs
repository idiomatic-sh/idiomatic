//! Request identity: corpus substitution and per-run generation.
use super::super::request_identity::service::RequestIdentity;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::files::types::InputReadFailure;
use std::sync::Arc;
const EDGE_REFUSAL: &str = "actual edge refusal";
fn corpus(path: FilePath) -> Vec<CorpusInput> {
    vec![CorpusInput::Unreadable {
        path,
        reason: InputReadFailure(EDGE_REFUSAL.into()),
    }]
}
/// An identity's corpus path, and the path of another corpus of the same size.
type Input = (&'static str, &'static str);

/// How an identity compares with its corpus, the other corpus, a shared handle and a later run.
#[derive(Debug, PartialEq)]
struct Compared {
    holds_its_corpus: bool,
    holds_the_other_corpus: bool,
    equals_its_shared_handle: bool,
    equals_a_later_run: bool,
}

type Value = Compared;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_inventory_size_does_not_authorize_corpus_substitution"),
        word: Word::Succeeds,
        input: || ("first", "second"),
        expect: Expect::Value(Compared {
            holds_its_corpus: true,
            holds_the_other_corpus: false,
            equals_its_shared_handle: true,
            equals_a_later_run: false,
        }),
    },
    Case {
        name: CaseName("each_run_has_a_distinct_generation_even_with_identical_metadata"),
        word: Word::Succeeds,
        input: || ("same", "same"),
        expect: Expect::Value(Compared {
            holds_its_corpus: true,
            holds_the_other_corpus: true,
            equals_its_shared_handle: true,
            equals_a_later_run: false,
        }),
    },
];

/// Builds an identity over the first corpus and a later one over the same corpus, and compares.
fn subject((path, other): Input) -> Result<Value, Failure> {
    let files = corpus(FilePath(path.into()));
    let first = Arc::new(RequestIdentity::new(&files));
    let shared = Arc::clone(&first);
    let later = Arc::new(RequestIdentity::new(&files));
    Ok(Compared {
        holds_its_corpus: first.corpus() == files.as_slice(),
        holds_the_other_corpus: first.corpus() == corpus(FilePath(other.into())).as_slice(),
        equals_its_shared_handle: first == shared,
        equals_a_later_run: first == later,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
