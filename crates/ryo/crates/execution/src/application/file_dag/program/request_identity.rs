//! One immutable listed corpus and execution generation, minted by composition
//! once per actual run. Prepared query programs may be reused; this identity may not.
use ryo_values::domain::files::types::CorpusInput;
use std::sync::Arc;

pub(super) struct RequestIdentity {
    corpus: Box<[CorpusInput]>,
    paths: Box<[ryo_language::domain::files::types::FilePath]>,
    generation: Arc<()>,
}
impl RequestIdentity {
    pub(super) fn new(corpus: &[CorpusInput]) -> Self {
        Self {
            corpus: corpus.to_vec().into_boxed_slice(),
            paths: corpus
                .iter()
                .map(|input| match input {
                    CorpusInput::Available(entry) | CorpusInput::ReadFailed { entry, .. } => {
                        entry.path.clone()
                    }
                    CorpusInput::Unreadable { path, .. } => path.clone(),
                })
                .collect(),
            generation: Arc::new(()),
        }
    }
    pub(super) fn paths(&self) -> &[ryo_language::domain::files::types::FilePath] {
        &self.paths
    }
    pub(super) fn matches_corpus(&self, corpus: &[CorpusInput]) -> bool {
        self.corpus.as_ref() == corpus
    }
    pub(super) fn path(
        &self,
        file: ryo_relations::domain::relations::types::FileId,
    ) -> Option<&ryo_language::domain::files::types::FilePath> {
        self.corpus.get(file.0 as usize).map(|input| match input {
            CorpusInput::Available(entry) | CorpusInput::ReadFailed { entry, .. } => &entry.path,
            CorpusInput::Unreadable { path, .. } => path,
        })
    }
    pub(super) fn contains(&self, file: ryo_relations::domain::relations::types::FileId) -> bool {
        (file.0 as usize) < self.corpus.len()
    }
}
impl PartialEq for RequestIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.generation, &other.generation)
    }
}
impl Eq for RequestIdentity {}

#[cfg(test)]
mod tests {
    use super::*;
    use ryo_language::domain::files::types::FilePath;
    use ryo_values::domain::files::types::InputReadFailure;
    fn corpus(path: &str) -> Vec<CorpusInput> {
        vec![CorpusInput::Unreadable {
            path: FilePath(path.into()),
            reason: InputReadFailure("actual edge refusal".into()),
        }]
    }
    #[test]
    fn equal_inventory_size_does_not_authorize_corpus_substitution() {
        let first = corpus("first.py");
        let identity = RequestIdentity::new(&first);
        assert!(identity.matches_corpus(&first));
        assert!(!identity.matches_corpus(&corpus("second.py")));
    }
    #[test]
    fn each_run_has_a_distinct_generation_even_with_identical_metadata() {
        let corpus = corpus("same.py");
        let first = Arc::new(RequestIdentity::new(&corpus));
        let same = Arc::clone(&first);
        let later = Arc::new(RequestIdentity::new(&corpus));
        assert!(first == same);
        assert!(first != later);
    }
}
