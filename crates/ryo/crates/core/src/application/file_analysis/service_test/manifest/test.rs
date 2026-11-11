//! The cases of discovery failures, primary manifest failures and project entry acquisition.

use crate::application::file_analysis::errors::FileAnalysisError;
use crate::application::file_analysis::fixtures::manifest::{CountingCorpus, Forgetful};
use crate::application::file_analysis::fixtures::visit::visited_inputs;
use crate::application::file_analysis::service::FileAnalysis;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::language::types::ProjectNotes;
use crate::outbound::language::types::ProjectToRead;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_execution_runtime::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution_runtime::outbound::facts::interface::FactsPort;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

fn discovery_failure_runs_without_parser_bytes_or_permission_invention()
-> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
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
    seen.push(format!("{:?}", worker.failure.is_none()));
    seen.push(format!("{:?}", corpus.reads.load(Ordering::Relaxed)));
    seen.push(format!("{:?}", language.parses.load(Ordering::Relaxed)));
    seen.push(format!("{:?}", worker.tables.bytes.0.is_empty()));
    for relation in [
        Relation::FilesystemFile,
        Relation::FilesystemExecutable,
        Relation::FilesystemLineCount,
        Relation::Function,
    ] {
        seen.push(format!(
            "{:?}",
            worker
                .tables
                .partials
                .iter()
                .any(|partial| partial.relation == relation)
        ));
    }
    seen.push(format!(
        "{:?}",
        !worker
            .tables
            .partials
            .iter()
            .any(|partial| partial.relation == Relation::FilesystemReadFailure)
    ));
    seen.push(format!("{:?}", worker.tables.fresh.is_empty()));
    seen.push(format!("{:?}", worker.tables.unrecorded));
    Ok(seen.join("|"))
}

fn primary_manifest_read_failure_retains_real_metadata_without_empty_project_source()
-> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
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
    seen.push(format!(
        "{:?}",
        service.read_manifests(&mut inputs).unwrap().is_empty()
    ));
    seen.push(format!("{:?}", corpus.reads.load(Ordering::Relaxed)));
    let CorpusInput::ReadFailed {
        entry: observed,
        reason,
    } = &inputs[0]
    else {
        panic!("retain primary read outcome");
    };
    seen.push(format!("{:?}", (observed) == (&entry)));
    seen.push(format!("{:?}", !reason.0.is_empty()));
    seen.push(format!(
        "{:?}",
        matches!(&inputs[1], CorpusInput::Available(observed) if observed == &companion)
    ));
    seen.push(format!(
        "{:?}",
        matches!(&inputs[2], CorpusInput::Unreadable { .. })
    ));
    Ok(seen.join("|"))
}

fn project_entry_acquisition_deduplicates_provider_candidates_before_observing()
-> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
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
    seen.push(format!("{:?}", observations.load(Ordering::Relaxed)));
    seen.push(format!(
        "{:?}",
        (result
            .observed_entries
            .iter()
            .map(|entry| entry.path.0.as_ref())
            .collect::<Vec<_>>())
            == (vec!["a.py", "z.py"])
    ));
    Ok(seen.join("|"))
}

/// A project answer that drops an entry the run observed for it is refused: the project made from it is not the one observed.
fn a_project_answer_that_drops_an_observed_entry_is_refused() -> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
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
    let projected = service.project_now(&input, &mut Symbols::default(), &mut *facts)?;
    seen.push(format!("{:?}", projected.refused.len()));
    Ok(seen.join("|"))
}

/// Reads a project whose language names one candidate entry and answers without the observation of it.
fn a_project_receipt_that_forgets_the_observed_entries_is_refused()
-> Result<String, FileAnalysisError> {
    let mut seen: Vec<String> = Vec::new();
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
    let projected = service.project_now(&input, &mut Symbols::default(), &mut *facts)?;
    seen.push(format!("{:?}", projected.refused.len()));
    Ok(seen.join("|"))
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `discovery_failure_runs_without_parser_bytes_or_permission_invention` scenario.
    DiscoveryFailureRunsWithoutParserBytesOrPermissionInvention,
    /// The `primary_manifest_read_failure_retains_real_metadata_without_empty_project_source` scenario.
    PrimaryManifestReadFailureRetainsRealMetadataWithoutEmptyProjectSource,
    /// The `project_entry_acquisition_deduplicates_provider_candidates_before_observing` scenario.
    ProjectEntryAcquisitionDeduplicatesProviderCandidatesBeforeObserving,
    /// The `a_project_answer_that_drops_an_observed_entry_is_refused` scenario.
    AProjectAnswerThatDropsAnObservedEntryIsRefused,
    /// The `a_project_receipt_that_forgets_the_observed_entries_is_refused` scenario.
    AProjectReceiptThatForgetsTheObservedEntriesIsRefused,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// What a scenario's run refuses.
type Failure = FileAnalysisError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("discovery_failure_runs_without_parser_bytes_or_permission_invention"),
        word: Word::Succeeds,
        input: || Given::DiscoveryFailureRunsWithoutParserBytesOrPermissionInvention,
        expect: Expect::Value(
            "true|0|0|true|true|true|true|true|true|true|[FilePath(\"missing.py\")]",
        ),
    },
    Case {
        name: CaseName(
            "primary_manifest_read_failure_retains_real_metadata_without_empty_project_source",
        ),
        word: Word::Succeeds,
        input: || Given::PrimaryManifestReadFailureRetainsRealMetadataWithoutEmptyProjectSource,
        expect: Expect::Value("true|1|true|true|true|true"),
    },
    Case {
        name: CaseName(
            "project_entry_acquisition_deduplicates_provider_candidates_before_observing",
        ),
        word: Word::Succeeds,
        input: || Given::ProjectEntryAcquisitionDeduplicatesProviderCandidatesBeforeObserving,
        expect: Expect::Value("2|true"),
    },
    Case {
        name: CaseName("a_project_answer_that_drops_an_observed_entry_is_refused"),
        word: Word::Refuses,
        input: || Given::AProjectAnswerThatDropsAnObservedEntryIsRefused,
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::ProjectObservationMismatch) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_project_receipt_that_forgets_the_observed_entries_is_refused"),
        word: Word::Refuses,
        input: || Given::AProjectReceiptThatForgetsTheObservedEntriesIsRefused,
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::ProjectObservationMismatch) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::DiscoveryFailureRunsWithoutParserBytesOrPermissionInvention => {
            discovery_failure_runs_without_parser_bytes_or_permission_invention()?
        }
        Given::PrimaryManifestReadFailureRetainsRealMetadataWithoutEmptyProjectSource => {
            primary_manifest_read_failure_retains_real_metadata_without_empty_project_source()?
        }
        Given::ProjectEntryAcquisitionDeduplicatesProviderCandidatesBeforeObserving => {
            project_entry_acquisition_deduplicates_provider_candidates_before_observing()?
        }
        Given::AProjectAnswerThatDropsAnObservedEntryIsRefused => {
            a_project_answer_that_drops_an_observed_entry_is_refused()?
        }
        Given::AProjectReceiptThatForgetsTheObservedEntriesIsRefused => {
            a_project_receipt_that_forgets_the_observed_entries_is_refused()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
