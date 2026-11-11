//! The cases of discovery failures, primary manifest failures and project entry acquisition.
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::visit::visited_inputs;
use crate::domain::render::types::Answering;
use crate::domain::visit::types::Keeping;
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::CorpusInventory;
use crate::outbound::corpus::types::PathStat;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::language::types::ProjectNotes;
use crate::outbound::language::types::ProjectToRead;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::project_context::types::EntryObservation;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::files::types::FileSize;
use ryo_values::domain::files::types::Nanoseconds;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
pub(crate) fn discovery_failure_runs_without_parser_bytes_or_permission_invention()
-> Result<usize, FileAnalysisError> {
    use crate::domain::{render::types::Answering, visit::types::Keeping};
    use ryo_values::domain::files::types::{CorpusInput, InputReadFailure};
    let corpus = Arc::new(CorpusFake::default());
    let input = CorpusInput::Unreadable {
        path: FilePath("missing.py".into()),
        reason: InputReadFailure("not found".into()),
    };
    let (worker, language) = visited_inputs(
        corpus.clone(),
        &[input],
        &[
            Relation::FilesystemFile,
            Relation::FilesystemReadFailure,
            Relation::FilesystemExecutable,
            Relation::FilesystemLineCount,
            Relation::Function,
        ],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Keeps,
        Arc::new(DigestFake),
        FactsFake::default().worker().unwrap(),
        None,
        1,
    )
    .unwrap();
    assert!(worker.failure.is_none());
    assert_eq!(
        corpus.reads.load(Ordering::Relaxed),
        0,
        "there is no fabricated entry to read"
    );
    assert_eq!(language.parses.load(Ordering::Relaxed), 0);
    assert!(worker.tables.bytes.0.is_empty());
    for relation in [
        Relation::FilesystemFile,
        Relation::FilesystemExecutable,
        Relation::FilesystemLineCount,
        Relation::Function,
    ] {
        assert!(
            worker
                .tables
                .partials
                .iter()
                .any(|partial| partial.relation == relation)
        );
    }
    assert!(
        !worker
            .tables
            .partials
            .iter()
            .any(|partial| partial.relation == Relation::FilesystemReadFailure)
    );
    assert!(worker.tables.fresh.is_empty());
    assert_eq!(worker.tables.unrecorded, [FilePath("missing.py".into())]);
    Ok(1)
}

pub(crate) fn primary_manifest_read_failure_retains_real_metadata_without_empty_project_source()
-> Result<usize, FileAnalysisError> {
    use ryo_values::domain::files::types::{
        CorpusEntry, CorpusInput, FileSize, InputReadFailure, Nanoseconds,
    };
    let corpus = Arc::new(CorpusFake::default());
    let service = FileAnalysis::new(
        corpus.clone(),
        Arc::new(LanguageFake::default()),
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let entry = CorpusEntry {
        executable: Default::default(),
        path: FilePath("pkg/project.manifest.fake".into()),
        size: FileSize(42),
        modified: Nanoseconds(19),
    };
    let companion = CorpusEntry {
        path: FilePath("input.rs".into()),
        ..entry.clone()
    };
    let mut inputs = [
        CorpusInput::Available(entry.clone()),
        CorpusInput::Available(companion.clone()),
        CorpusInput::Unreadable {
            path: FilePath("missing.manifest.fake".into()),
            reason: InputReadFailure("not found".into()),
        },
    ];
    assert!(
        service.read_manifests(&mut inputs).unwrap().is_empty(),
        "a failed read never becomes empty manifest text"
    );
    assert_eq!(
        corpus.reads.load(Ordering::Relaxed),
        1,
        "only the available primary manifest is read; no consulted/source path is substituted"
    );
    let CorpusInput::ReadFailed {
        entry: observed,
        reason,
    } = &inputs[0]
    else {
        panic!("retain primary read outcome");
    };
    assert_eq!(observed, &entry);
    assert!(!reason.0.is_empty());
    assert!(matches!(&inputs[1], CorpusInput::Available(observed) if observed == &companion));
    assert!(matches!(&inputs[2], CorpusInput::Unreadable { .. }));
    Ok(1)
}

pub(crate) fn project_entry_acquisition_deduplicates_provider_candidates_before_observing()
-> Result<usize, FileAnalysisError> {
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

pub(crate) struct CountingCorpus {
    inner: CorpusFake,
    observations: Arc<AtomicUsize>,
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

/// A language that answers the project without the entries it was given to observe.
struct Forgetful(LanguageFake);

impl LanguagePort for Forgetful {
    fn relations(
        &self,
    ) -> Result<
        crate::outbound::language::types::RelationList,
        crate::outbound::language::errors::LanguageError,
    > {
        self.0.relations()
    }
    fn linked(&self) -> crate::outbound::language::types::LinkedNames {
        self.0.linked()
    }
    fn kind_vocabularies(&self) -> ryo_values::domain::descriptions::types::KindVocabularies {
        self.0.kind_vocabularies()
    }
    fn unknown_reasons(&self) -> crate::outbound::language::types::UnknownReasons {
        self.0.unknown_reasons()
    }
    fn supports(&self, relation: Relation) -> crate::outbound::language::types::Answers {
        self.0.supports(relation)
    }
    fn reads_manifest(&self, path: &FilePath) -> crate::outbound::language::types::ManifestReading {
        self.0.reads_manifest(path)
    }
    fn parse(
        &self,
        file: crate::outbound::language::types::FileToRead<'_>,
        table: &mut ryo_language::domain::elements::types::Elements,
        symbols: &mut Symbols,
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) -> crate::outbound::language::types::FileReading {
        self.0.parse(file, table, symbols, out)
    }
    fn project_entries(&self, input: &ProjectToRead<'_>) -> Vec<FilePath> {
        self.0.project_entries(input)
    }
    fn project(
        &self,
        input: &ProjectToRead<'_>,
        symbols: &mut Symbols,
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) -> Result<
        crate::outbound::language::types::ProjectAnswer,
        crate::outbound::language::errors::LanguageError,
    > {
        let mut answer = self.0.project(input, symbols, out)?;
        answer.observed_entries.clear();
        Ok(answer)
    }
}

/// A project answer that drops an entry the run observed for it is refused: the project made from it is not the one observed.
pub(crate) fn a_project_answer_that_drops_an_observed_entry_is_refused()
-> Result<usize, FileAnalysisError> {
    let service = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::new(Forgetful(LanguageFake {
            project_entry_paths: vec![FilePath("setup.cfg".into())],
            ..LanguageFake::default()
        })),
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let input = ProjectToRead {
        search_scope: &Default::default(),
        given_inventory: &Default::default(),
        observed_entries: &Default::default(),
        repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
        root: &RootPath(".".into()),
        manifests: &[],
        files: &[],
        notes: &ProjectNotes(Vec::new()),
        wanted: &[],
    };
    let mut facts = FactsFake::default().worker()?;
    service.project_now(&input, &mut Symbols::default(), &mut *facts)?;
    Ok(0)
}

/// Reads a project whose language names one candidate entry and answers without the observation of it.
pub(crate) fn a_receipt_that_forgets_the_entries_is_refused() -> Result<usize, FileAnalysisError> {
    let service = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::new(LanguageFake {
            project_entry_paths: vec![FilePath("a.py".into())],
            forgets_entries: true,
            ..LanguageFake::default()
        }),
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let input = ProjectToRead {
        search_scope: &Default::default(),
        given_inventory: &Default::default(),
        observed_entries: &Default::default(),
        repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
        root: &RootPath(".".into()),
        manifests: &[],
        files: &[],
        notes: &ProjectNotes(Vec::new()),
        wanted: &[],
    };
    let mut facts = FactsFake::default().worker()?;
    service
        .project_now(&input, &mut Symbols::default(), &mut *facts)
        .map(|_| 0)
}
