//! The `request_identity` application operation.

use super::types::CorpusMembership;
use ryo_values::domain::files::types::CorpusInput;
use std::sync::Arc;

pub(crate) struct RequestIdentity {
    corpus: Box<[CorpusInput]>,
    paths: Box<[ryo_language::domain::files::types::FilePath]>,
    generation: Arc<()>,
    available: std::collections::BTreeMap<
        ryo_language::domain::files::types::FilePath,
        Option<ryo_relations::domain::relations::types::FileId>,
    >,
}

impl RequestIdentity {
    pub(crate) fn new(corpus: &[CorpusInput]) -> Self {
        // Path-based applicability cannot distinguish duplicate corpus entries.
        // Retain ambiguity explicitly so no duplicate can discharge transport.
        let mut available = std::collections::BTreeMap::new();
        for (at, input) in corpus.iter().enumerate() {
            let path = match input {
                CorpusInput::Available(entry) | CorpusInput::ReadFailed { entry, .. } => {
                    &entry.path
                }
                CorpusInput::Unreadable { path, .. } => path,
            };
            let file = match input {
                CorpusInput::Available(_) => match u32::try_from(at) {
                    Ok(index) => Some(ryo_relations::domain::relations::types::FileId(index)),
                    Err(_) => None,
                },
                CorpusInput::ReadFailed { .. } | CorpusInput::Unreadable { .. } => None,
            };
            available
                .entry(path.clone())
                .and_modify(|entry| *entry = None)
                .or_insert(file);
        }
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
            available,
        }
    }
    pub(crate) fn available_file(
        &self,
        path: &ryo_language::domain::files::types::FilePath,
    ) -> Option<ryo_relations::domain::relations::types::FileId> {
        self.available.get(path).copied().flatten()
    }
    pub(crate) fn paths(&self) -> &[ryo_language::domain::files::types::FilePath] {
        &self.paths
    }
    pub(crate) fn corpus(&self) -> &[CorpusInput] {
        &self.corpus
    }
    pub(crate) fn path(
        &self,
        file: ryo_relations::domain::relations::types::FileId,
    ) -> Option<&ryo_language::domain::files::types::FilePath> {
        let listed = usize::try_from(file.0).map_or(None, |at| self.corpus.get(at));
        listed.map(|input| match input {
            CorpusInput::Available(entry) | CorpusInput::ReadFailed { entry, .. } => &entry.path,
            CorpusInput::Unreadable { path, .. } => path,
        })
    }
    pub(crate) fn contains(
        &self,
        file: ryo_relations::domain::relations::types::FileId,
    ) -> CorpusMembership {
        if usize::try_from(file.0).is_ok_and(|at| at < self.corpus.len()) {
            CorpusMembership::Listed
        } else {
            CorpusMembership::Unlisted
        }
    }
}

impl PartialEq for RequestIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.generation, &other.generation)
    }
}

impl Eq for RequestIdentity {}
