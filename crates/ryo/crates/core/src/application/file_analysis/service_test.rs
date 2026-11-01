//! Analysis uses its injected language and propagates required failures.
use super::errors::FileAnalysisError;
use super::service::{FileAnalysis, recovery_of};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::digest::fixtures::fake::DigestFake;
use crate::outbound::facts::fixtures::fake::FactsFake;
use crate::outbound::facts::interface::FactsPort as _;

use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::language::types::{ProjectInput, ProjectNotes};
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_language::inbound::use_cases::project::types::{ManifestFile, ManifestText};
use std::sync::{Arc, atomic::Ordering};

#[test]
fn only_incomplete_diagnostics_mark_code_relations_as_recovery() {
    use crate::domain::coverage::service::partial_of;
    use crate::domain::coverage::types::Recovery;
    use crate::outbound::language::types::FileReading;
    use ryo_language::domain::elements::service::{
        diagnose, diagnose_with_metadata, diagnostics, has_incomplete_syntax, table_builder,
    };
    use ryo_language::domain::elements::types::{
        DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, Elements, TreeImpact,
    };
    use ryo_language::domain::relations::types::{Offset, Relation, ReportText};
    use ryo_language::domain::symbols::types::ByteSlice;

    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    let source = ByteSlice(b"x");
    let mut builder = table_builder(&mut elements, &mut symbols, source);
    diagnose_with_metadata(
        &mut builder,
        DiagnosticMetadata::new(
            DiagnosticPhase::Validation,
            TreeImpact::Preserved,
            DiagnosticCode::provider_owned("fixture.target-syntax"),
        ),
        ReportText("syntax is unsupported by this target"),
        Offset(0),
        Offset(1),
    );
    drop(builder);

    let preserved = recovery_of(
        &FileReading::Read(ryo_values::domain::stored_facts::types::LanguageNumber(0)),
        &elements,
    );
    assert_eq!(preserved, Recovery::Whole);
    assert!(!has_incomplete_syntax(&elements));
    assert_eq!(
        diagnostics(&elements).count(),
        1,
        "diagnostic remains reportable"
    );
    assert!(partial_of(&[], preserved, &[Relation::Function]).is_empty());

    let mut builder = table_builder(&mut elements, &mut symbols, source);
    diagnose(
        &mut builder,
        ReportText("legacy parser diagnostic without impact metadata"),
        Offset(0),
        Offset(1),
    );
    drop(builder);
    let incomplete = recovery_of(
        &FileReading::Read(ryo_values::domain::stored_facts::types::LanguageNumber(0)),
        &elements,
    );
    assert_eq!(incomplete, Recovery::SyntaxErrors);
    assert!(has_incomplete_syntax(&elements));
    assert_eq!(diagnostics(&elements).count(), 1);
    assert!(!partial_of(&[], incomplete, &[Relation::Function]).is_empty());
}

/// What the last run kept, for a run that uses nothing of it.
static NOTHING_STORED: std::sync::LazyLock<ryo_values::domain::stores::types::Stored> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::Stored::default);

/// What the files of the last run handed, for a run that uses nothing of it.
static NOTHING_HANDED: std::sync::LazyLock<ryo_values::domain::stores::types::StoredHanded> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::StoredHanded::default);

/// No bucket of what the files handed, for a run that reads none.
static NOTHING_WANTED: ryo_values::domain::stores::types::BucketsWanted =
    ryo_values::domain::stores::types::BucketsWanted::These(Vec::new());

type Input = &'static str;
type Value = usize;
type Failure = FileAnalysisError;

fn subject(path: Input) -> Result<Value, Failure> {
    if path == "invalid-provider.rs" {
        return invalid_provider();
    }
    if path == "manifest-and-stray" {
        return manifest_and_stray();
    }
    if path == "digests" {
        return digested();
    }
    let language = Arc::new(LanguageFake::default());
    let service = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::<LanguageFake>::clone(&language),
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let input = ProjectInput {
        repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
        root: &RootPath(".".into()),
        manifests: &[ManifestFile {
            path: FilePath(path.into()),
            text: ManifestText("".into()),
        }],
        files: &[],
        notes: &ProjectNotes(Vec::new()),
        wanted: &[],
    };
    let mut facts = FactsFake::default().worker()?;
    let projected = service.project_now(&input, &mut Symbols::default(), &mut *facts)?;
    assert!(
        projected.refused.is_empty(),
        "a manifest the language answers is not refused"
    );
    Ok(language.projects.load(Ordering::Relaxed))
}

/// Visits each file at `paths` of a fake corpus, the way a lint does, with the language `language`: the worker that visited them, and
/// the language to ask what it was given.
fn visited(
    paths: &[&str],
    language: LanguageFake,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    let answering =
        crate::domain::render::types::Answering::Rows(crate::domain::render::types::RowsAnswer {
            limit: ryo_values::domain::records::types::RowTotal(0),
            headings: Vec::new(),
            columns: crate::domain::render::types::AnswerColumns::One,
        });
    visited_as(
        paths,
        language,
        &answering,
        crate::domain::visit::types::Keeping::Keeps,
        Arc::new(DigestFake),
    )
}

/// Visits each file at `paths` as `visited` does, answering as `answering` says, over a store that keeps as `keeping` says and with
/// the digests `digest`.
fn visited_as(
    paths: &[&str],
    language: LanguageFake,
    answering: &crate::domain::render::types::Answering,
    keeping: crate::domain::visit::types::Keeping,
    digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    use super::types::{FileVisit, Worker};
    use crate::domain::lowering::types::Counting;
    use crate::domain::operator::types::TreeSteps;
    use crate::domain::placement::types::{Mask, PlacedScopes, ScopePlacement};
    use crate::domain::visit::types::{Noting, RoleRead};
    use crate::domain::workers::service::tables_for;
    use crate::domain::workers::types::VisitSetting;
    use crate::outbound::corpus::interface::CorpusPort as _;
    use crate::outbound::matching::fixtures::fake::MatchingFake;
    use crate::outbound::matching::interface::MatchingPort as _;
    use crate::outbound::scheduler::types::JobNumber;
    use ryo_language::domain::relations::types::{Relation, WorkerNumber};
    use ryo_values::domain::files::types::{CorpusFile, FileBytes};
    use ryo_values::domain::runs::types::MemoryBytes;
    let corpus = Arc::new(CorpusFake {
        files: paths
            .iter()
            .map(|path| CorpusFile {
                path: FilePath((*path).into()),
                bytes: FileBytes(b"fn main() {}".to_vec()),
            })
            .collect(),
        ..CorpusFake::default()
    });
    let files = corpus.inventory().expect("fake corpus inventory");
    let language = Arc::new(language);
    let service = FileAnalysis::new(
        corpus,
        Arc::<LanguageFake>::clone(&language),
        digest,
        Arc::new(MemoryFake::default()),
    );
    let placed = PlacedScopes {
        placement: ScopePlacement {
            always: Mask(vec![1]),
            ..ScopePlacement::default()
        },
        matcher: MatchingFake.compile(&[]).expect("empty glob set"),
    };
    let visit = FileVisit {
        setting: VisitSetting {
            unknown: &[],
            plans: &[],
            bindings: &[],
            wanted: &[Relation::Function],
            paths: &[],
            first_bucket: &[],
            ids: &[],
            placed: &placed,
            records: &[],
            replaying: crate::domain::workers::types::Replaying::Findings,
            stored: &NOTHING_STORED,
            handed: &NOTHING_HANDED,
            wanted_buckets: &NOTHING_WANTED,
            budget: MemoryBytes(u64::MAX),
            noted: Noting::Unwanted,
            roled: RoleRead::Skips,
            needs: &[],
            answering,
            keeping,
        },
    };
    let mut worker = Worker {
        tables: tables_for(
            WorkerNumber(0),
            &[],
            TreeSteps::Lowered,
            Counting::DoesNotCount,
        ),
        facts: FactsFake::default().worker()?,
        failure: None,
    };
    for index in 0..u32::try_from(files.0.len()).unwrap_or(u32::MAX) {
        service.visit_file(&mut worker, &visit, &files.0, JobNumber(index));
    }
    Ok((worker, language))
}

/// Visit a real fake file whose injected language emits an invalid node reference.
fn invalid_provider() -> Result<Value, Failure> {
    let (worker, language) = visited(
        &["invalid-provider.rs"],
        LanguageFake {
            out_of_range: true,
            ..LanguageFake::default()
        },
    )?;
    assert_eq!(language.parses.load(Ordering::Relaxed), 1);
    worker.failure.map_or(Ok(0), Err)
}

/// Visit a source file, a manifest the language reads and a stray file no language reads: whether the one file left unread is the stray one.
fn manifest_and_stray() -> Result<Value, Failure> {
    let (worker, _) = visited(
        &["a.rs", "project.manifest.fake", "stray.xyz"],
        LanguageFake::default(),
    )?;
    Ok(usize::from(
        worker.failure.is_none()
            && worker.tables.unread.len() == 1
            && worker
                .tables
                .unread
                .iter()
                .all(|file| &*file.path.0 == "stray.xyz"),
    ))
}

/// How many times a run that keeps its findings digests two files read over a store that keeps nothing (tens), and over one that
/// keeps (ones, at least one): none, then some.
fn digested() -> Result<Value, Failure> {
    let mut counts = Vec::new();
    for keeping in [
        crate::domain::visit::types::Keeping::Discards,
        crate::domain::visit::types::Keeping::Keeps,
    ] {
        let digest = Arc::new(crate::outbound::digest::fixtures::fake::DigestCounting::default());
        let (worker, _) = visited_as(
            &["a.rs", "b.rs"],
            LanguageFake::default(),
            &crate::domain::render::types::Answering::Findings,
            keeping,
            Arc::<crate::outbound::digest::fixtures::fake::DigestCounting>::clone(&digest),
        )?;
        assert!(worker.failure.is_none(), "the files are read");
        assert_eq!(worker.tables.fresh.len(), 2, "and their findings recorded");
        counts.push(digest.digests.load(Ordering::Relaxed));
    }
    Ok(counts.first().copied().unwrap_or(1) * 10
        + usize::from(counts.get(1).is_some_and(|each| *each > 0)))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_run_whose_store_keeps_nothing_digests_no_file"),
        word: Word::Succeeds,
        input: || "digests",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("invalid_injected_provider_rows_prevent_an_analysis_result"),
        word: Word::Refuses,
        input: || "invalid-provider.rs",
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::Provider { code: crate::domain::admission::errors::AdmissionError::NodeOutsideTable, provider } if provider == "fake ")
            {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("the_injected_language_performs_project_analysis"),
        word: Word::Succeeds,
        input: || "manifest.fake",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_lint_leaves_unread_the_stray_file_and_not_the_manifest_a_language_reads"),
        word: Word::Succeeds,
        input: || "manifest-and-stray",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_required_language_failure_prevents_an_analysis_result"),
        word: Word::Fails,
        input: || "broken.manifest.fake",
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::Language(_)) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
