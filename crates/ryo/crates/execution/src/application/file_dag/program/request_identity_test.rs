//! Request identity: corpus substitution and per-run generation.
use super::request_identity::RequestIdentity;
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
#[test]
fn equal_inventory_size_does_not_authorize_corpus_substitution() {
    let first = corpus(FilePath(FIRST.into()));
    let identity = RequestIdentity::new(&first);
    assert_eq!(identity.corpus(), first.as_slice());
    assert_ne!(
        identity.corpus(),
        corpus(FilePath(SECOND.into())).as_slice()
    );
}
#[test]
fn each_run_has_a_distinct_generation_even_with_identical_metadata() {
    let corpus = corpus(FilePath(SAME.into()));
    let first = Arc::new(RequestIdentity::new(&corpus));
    let same = Arc::clone(&first);
    let later = Arc::new(RequestIdentity::new(&corpus));
    assert!(first == same);
    assert!(first != later);
}
