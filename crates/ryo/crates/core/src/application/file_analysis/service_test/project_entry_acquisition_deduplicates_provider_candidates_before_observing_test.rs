//! The scenario `project_entry_acquisition_deduplicates_provider_candidates_before_observing` of the cases of this file.

use super::*;

pub(super) fn project_entry_acquisition_deduplicates_provider_candidates_before_observing() -> Result<Value, Failure> {
    use project_entry_acquisition_deduplicates_provider_candidates_before_observing_doubles::CountingCorpus;
    let observations = Arc::new(AtomicUsize::new(0));
    let corpus = Arc::new(CountingCorpus {
        inner: CorpusFake::default(),
        observations: Arc::clone(&observations),
    });
    let language = Arc::new(LanguageFake {
        project_entry_paths: vec![
            FilePath("z.py".into()),
            FilePath("a.py".into()),
            FilePath("a.py".into()),
        ],
        ..LanguageFake::default()
    });
    let service = FileAnalysis::new(
        corpus,
        language,
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let root = RootPath(".".into());
    let scope = Default::default();
    let inventory = Default::default();
    let notes = ProjectNotes::default();
    let input = ProjectToRead {
        root: &root,
        repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
        search_scope: &scope,
        given_inventory: &inventory,
        observed_entries: &Default::default(),
        manifests: &[],
        files: &[],
        notes: &notes,
        wanted: &[],
    };
    let mut facts = FactsFake::default().worker().expect("facts worker");
    let result = service
        .project_now(&input, &mut Symbols::default(), &mut *facts)
        .expect("project");
    assert_eq!(observations.load(Ordering::Relaxed), 2);
    assert_eq!(
        result
            .observed_entries
            .iter()
            .map(|entry| entry.path.0.as_ref())
            .collect::<Vec<_>>(),
        vec!["a.py", "z.py"]
    );
    Ok(1)
}

mod project_entry_acquisition_deduplicates_provider_candidates_before_observing_doubles {
    use super::*;
    use crate::outbound::corpus::{
        errors::CorpusError,
        interface::CorpusPort,
        types::{CorpusInventory, PathStat},
    };
    use ryo_language::domain::project_context::types::EntryObservation;
    use ryo_values::domain::files::types::{CorpusEntry, FileBytes};
    use std::sync::atomic::AtomicUsize;

    pub(super) struct CountingCorpus {
        pub(super) inner: CorpusFake,
        pub(super) observations: Arc<AtomicUsize>,
    }

    impl CorpusPort for CountingCorpus {
        fn inventory(&self) -> Result<CorpusInventory, CorpusError> {
            self.inner.inventory()
        }
        fn read(&self, entry: &CorpusEntry, into: &mut FileBytes) -> Result<(), CorpusError> {
            self.inner.read(entry, into)
        }
        fn stat(&self, path: &FilePath) -> Result<PathStat, CorpusError> {
            self.inner.stat(path)
        }
        fn observe_entry(&self, path: &FilePath) -> EntryObservation {
            self.observations.fetch_add(1, Ordering::Relaxed);
            self.inner.observe_entry(path)
        }
    }
}
