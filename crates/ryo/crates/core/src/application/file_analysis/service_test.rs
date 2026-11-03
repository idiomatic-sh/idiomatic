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

use crate::application::file_dag::settings::VisitSetting;
use crate::domain::lowering::types::Counting;
use crate::domain::operator::types::TreeSteps;
use crate::domain::placement::types::{Mask, PlacedScopes, ScopePlacement};
use crate::domain::trust::types::{Needs, RelationRead};
use crate::domain::workers::service::tables_for;
use crate::outbound::matching::interface::MatchingPort as _;
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_language::domain::relations::types::{Relation, WorkerNumber};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::{Emit, Plan};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};

#[test]
fn syntax_applicability_skips_only_gated_rules_and_names_the_reason() {
    let plan = Plan {
        parameters: Vec::new(),
        steps: Vec::new(),
        comparison_origins: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    };
    let plans = [plan.clone(), plan];
    let ids = [RuleId("gated".into()), RuleId("always".into())];
    let paths = [FilePath("source.py".into())];
    let applicability = [RuleApplicability::AcceptedSyntax, RuleApplicability::Always];
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: crate::outbound::matching::fixtures::fake::MatchingFake
            .compile(&[])
            .expect("empty matcher"),
    };
    let setting = VisitSetting {
        source_language: None,
        executor: &crate::application::file_dag::execution::CommonOperatorEngine,
        operators: crate::application::file_dag::testing::request_fixture(&plans, &[], &[]),
        domain: crate::domain::workers::types::VisitSetting {
            plans: &plans,
            bindings: &[],
            wanted: &[Relation::Function, Relation::Identifier],
            paths: &paths,
            first_bucket: &[],
            ids: &ids,
            applicability: &applicability,
            applicability_can_gate: true,
            placed: &placed,
            records: &[],
            replaying: crate::domain::workers::types::Replaying::Findings,
            stored: &NOTHING_STORED,
            handed: &NOTHING_HANDED,
            wanted_buckets: &NOTHING_WANTED,
            budget: ryo_values::domain::runs::types::MemoryBytes(u64::MAX),
            noted: crate::domain::visit::types::Noting::Unwanted,
            roled: crate::domain::visit::types::RoleRead::Skips,
            needs: &[
                Needs {
                    reads: vec![RelationRead {
                        relation: Relation::Function,
                        kinds: Vec::new(),
                    }],
                    ..Needs::default()
                },
                Needs {
                    reads: vec![RelationRead {
                        relation: Relation::Identifier,
                        kinds: Vec::new(),
                    }],
                    ..Needs::default()
                },
            ],
            answering: &crate::domain::render::types::Answering::Findings,
            keeping: crate::domain::visit::types::Keeping::Discards,
            unknown: &[],
        },
    };
    for (acceptance, expected_reason) in [
        (SyntaxAcceptance::Rejected, "rule skipped: syntax rejected"),
        (
            SyntaxAcceptance::Undetermined,
            "rule skipped: syntax acceptance undetermined",
        ),
    ] {
        assert_gated_acceptance(&setting, acceptance, expected_reason);
    }
    assert_accepted_and_missing_applicability(&setting);
}

/// Check a rejected or undetermined source without mixing its owner tables with other cases.
fn assert_gated_acceptance(
    setting: &VisitSetting<'_>,
    acceptance: SyntaxAcceptance,
    expected_reason: &str,
) {
    let mut tables = tables_for(
        WorkerNumber(0),
        setting.plans,
        TreeSteps::Lowered,
        Counting::DoesNotCount,
    );
    tables.mask = Mask(vec![0b11]);
    let skipped = FileAnalysis::gate_rules(
        &mut tables,
        setting,
        ryo_language::domain::relations::types::FileId(0),
        acceptance,
    );
    assert_eq!(skipped, [true, false]);
    assert_eq!(tables.mask.0, [0b10]);
    assert_eq!(
        FileAnalysis::active_wanted(&tables, setting, acceptance).as_ref(),
        [Relation::Identifier]
    );
    if acceptance == SyntaxAcceptance::Rejected {
        assert!(tables.unknowns.is_empty());
        assert_eq!(tables.skipped.len(), 1);
        assert_eq!(tables.skipped[0].rule, setting.ids[0]);
        assert_eq!(tables.skipped[0].path, setting.paths[0]);
        assert_eq!(
            tables.skipped[0].reason,
            ryo_values::domain::runs::types::SkipReason::SyntaxRejected
        );
        assert!(expected_reason.contains("rejected"));
    } else {
        assert_eq!(tables.unknowns.len(), 1);
        assert_eq!(tables.unknowns[0].rule, setting.ids[0]);
        assert_eq!(tables.unknowns[0].path, setting.paths[0]);
        assert_eq!(&*tables.unknowns[0].message.0, expected_reason);
        assert!(tables.skipped.is_empty());
    }
}

/// Accepted syntax keeps enabled rules; absent applicability metadata still records unknowns.
fn assert_accepted_and_missing_applicability(setting: &VisitSetting<'_>) {
    let mut tables = tables_for(
        WorkerNumber(0),
        setting.plans,
        TreeSteps::Lowered,
        Counting::DoesNotCount,
    );
    tables.mask = Mask(vec![0b11]);
    assert_eq!(
        FileAnalysis::gate_rules(
            &mut tables,
            setting,
            ryo_language::domain::relations::types::FileId(0),
            SyntaxAcceptance::Accepted,
        ),
        [false, false]
    );
    assert_eq!(tables.mask.0, [0b11]);
    assert_eq!(
        FileAnalysis::active_wanted(&tables, setting, SyntaxAcceptance::Accepted).as_ref(),
        [Relation::Function, Relation::Identifier]
    );
    let always_setting = VisitSetting {
        source_language: None,
        executor: &crate::application::file_dag::execution::CommonOperatorEngine,
        operators: crate::application::file_dag::testing::request_fixture(
            &setting.plans[1..],
            &[],
            &[],
        ),
        domain: crate::domain::workers::types::VisitSetting {
            plans: &setting.plans[1..],
            ids: &setting.ids[1..],
            applicability: &setting.applicability[1..],
            applicability_can_gate: false,
            needs: &setting.needs[1..],
            ..(setting.clone()).domain
        },
        ..setting.clone()
    };
    assert!(matches!(
        FileAnalysis::active_wanted(&tables, &always_setting, SyntaxAcceptance::Rejected),
        std::borrow::Cow::Borrowed(_)
    ));
    assert!(!FileAnalysis::may_gate_rules(
        setting,
        SyntaxAcceptance::Accepted
    ));
    assert!(FileAnalysis::may_gate_rules(
        setting,
        SyntaxAcceptance::Rejected
    ));
    assert!(tables.unknowns.is_empty());

    let missing_policy = VisitSetting {
        domain: crate::domain::workers::types::VisitSetting {
            applicability: &[],
            applicability_can_gate: true,
            ..(setting.clone()).domain
        },
        ..setting.clone()
    };
    assert!(FileAnalysis::may_gate_rules(
        &missing_policy,
        SyntaxAcceptance::Accepted
    ));
    let mut missing_tables = tables_for(
        WorkerNumber(0),
        setting.plans,
        TreeSteps::Lowered,
        Counting::DoesNotCount,
    );
    missing_tables.mask = Mask(vec![0b11]);
    assert_eq!(
        FileAnalysis::gate_rules(
            &mut missing_tables,
            &missing_policy,
            ryo_language::domain::relations::types::FileId(0),
            SyntaxAcceptance::Accepted,
        ),
        [true, true]
    );
    assert_eq!(missing_tables.mask.0, [0]);
    assert!(
        FileAnalysis::active_wanted(&missing_tables, &missing_policy, SyntaxAcceptance::Accepted)
            .is_empty()
    );
    assert_eq!(missing_tables.unknowns.len(), 2);
    assert!(missing_tables.skipped.is_empty());
    assert!(missing_tables.unknowns.iter().all(|unknown| {
        &*unknown.message.0 == "rule skipped: applicability metadata unavailable"
    }));
}

#[test]
fn only_incomplete_diagnostics_mark_code_relations_as_recovery() {
    use crate::domain::coverage::service::partial_of;
    use crate::domain::coverage::types::Recovery;
    use crate::outbound::language::types::FileReading;
    use ryo_language::domain::elements::service::{
        diagnose, diagnose_with_metadata, diagnostics, has_incomplete_lexical_stream,
        has_incomplete_syntax, table_builder,
    };
    use ryo_language::domain::elements::types::{
        DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, Elements, TreeImpact,
    };
    use ryo_language::domain::relations::types::{Offset, ReportText};
    use ryo_language::domain::symbols::types::ByteSlice;

    let source = ByteSlice(b"x");
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    {
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
    }
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

    let mut lexical_elements = Elements::default();
    let mut lexical_symbols = Symbols::default();
    {
        let mut lexical_builder =
            table_builder(&mut lexical_elements, &mut lexical_symbols, source);
        diagnose_with_metadata(
            &mut lexical_builder,
            DiagnosticMetadata::new(
                DiagnosticPhase::Parse,
                TreeImpact::LexicallyComplete,
                DiagnosticCode::provider_owned("fixture.parser-recovered"),
            ),
            ReportText("invalid syntax, complete token stream"),
            Offset(0),
            Offset(1),
        );
    }
    let lexical_recovery = recovery_of(
        &FileReading::Read(ryo_values::domain::stored_facts::types::LanguageNumber(0)),
        &lexical_elements,
    );
    assert_eq!(lexical_recovery, Recovery::LexicallyCompleteSyntaxErrors);
    assert!(has_incomplete_syntax(&lexical_elements));
    assert!(!has_incomplete_lexical_stream(&lexical_elements));
    let lexical_partials = partial_of(
        &[],
        lexical_recovery,
        &[
            Relation::Comment,
            Relation::IndentRegion,
            Relation::Function,
        ],
    );
    assert_eq!(lexical_partials.len(), 1);
    assert_eq!(lexical_partials[0].relation, Relation::Function);
    let mut incomplete_elements = Elements::default();
    let mut incomplete_symbols = Symbols::default();
    {
        let mut validation_builder =
            table_builder(&mut incomplete_elements, &mut incomplete_symbols, source);
        diagnose(
            &mut validation_builder,
            ReportText("legacy parser diagnostic without impact metadata"),
            Offset(0),
            Offset(1),
        );
    }
    let incomplete = recovery_of(
        &FileReading::Read(ryo_values::domain::stored_facts::types::LanguageNumber(0)),
        &incomplete_elements,
    );
    assert_eq!(incomplete, Recovery::SyntaxErrors);
    assert!(has_incomplete_syntax(&incomplete_elements));
    assert!(has_incomplete_lexical_stream(&incomplete_elements));
    assert_eq!(diagnostics(&incomplete_elements).count(), 1);
    assert!(!partial_of(&[], incomplete, &[Relation::Function]).is_empty());
}

/// What the last run kept, for a run that uses nothing of it.
static NOTHING_STORED: std::sync::LazyLock<ryo_values::domain::stores::types::Stored> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::Stored::default);

/// What the files of the last run handed, for a run that uses nothing of it.
static NOTHING_HANDED: std::sync::LazyLock<ryo_values::domain::stores::types::ReadHanded> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::ReadHanded::default);

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
    visited_as_with_facts(
        paths,
        language,
        answering,
        keeping,
        digest,
        FactsFake::default().worker()?,
    )
}

fn visited_as_with_facts(
    paths: &[&str],
    language: LanguageFake,
    answering: &crate::domain::render::types::Answering,
    keeping: crate::domain::visit::types::Keeping,
    digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
    facts: Box<dyn crate::outbound::facts::interface::FactsWorker>,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    use super::types::{FileVisit, Worker};
    use crate::domain::visit::types::{Noting, RoleRead};
    use crate::outbound::corpus::interface::CorpusPort as _;
    use crate::outbound::matching::fixtures::fake::MatchingFake;
    use crate::outbound::matching::interface::MatchingPort as _;
    use crate::outbound::scheduler::types::JobNumber;
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
        store: None,
        setting: VisitSetting {
            source_language: None,
            executor: &crate::application::file_dag::execution::CommonOperatorEngine,
            operators: crate::application::file_dag::testing::request_fixture(&[], &[], &[]),
            domain: crate::domain::workers::types::VisitSetting {
                unknown: &[],
                plans: &[],
                bindings: &[],
                wanted: &[Relation::Function],
                paths: &[],
                first_bucket: &[],
                ids: &[],
                applicability: &[],
                applicability_can_gate: true,
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
        },
    };
    let mut worker = Worker {
        tables: tables_for(
            WorkerNumber(0),
            &[],
            TreeSteps::Lowered,
            Counting::DoesNotCount,
        ),
        facts,
        old_rows: None,
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

#[test]
fn required_dependency_stat_and_read_errors_cannot_disappear() {
    use crate::outbound::corpus::{
        errors::CorpusError,
        interface::CorpusPort,
        types::{Inventory, PathStat},
    };
    use ryo_values::domain::files::types::{CorpusEntry, FileBytes, FileSize, Nanoseconds};
    struct Refuses {
        stat: bool,
    }
    impl CorpusPort for Refuses {
        fn inventory(&self) -> Result<Inventory, CorpusError> {
            Ok(Inventory(vec![]))
        }
        fn read(&self, _: &CorpusEntry, _: &mut FileBytes) -> Result<(), CorpusError> {
            Err(CorpusError::Unreadable {
                path: "required".into(),
                reason: "read denied".into(),
            })
        }
        fn stat(&self, path: &FilePath) -> Result<PathStat, CorpusError> {
            if self.stat {
                Err(CorpusError::Unreadable {
                    path: path.0.to_string(),
                    reason: "stat denied".into(),
                })
            } else {
                Ok(PathStat::Found(CorpusEntry {
                    path: path.clone(),
                    size: FileSize(1),
                    modified: Nanoseconds(1),
                }))
            }
        }
    }
    for stat in [false, true] {
        let service = FileAnalysis::new(
            Arc::new(Refuses { stat }),
            Arc::new(LanguageFake::default()),
            Arc::new(DigestFake),
            Arc::new(MemoryFake::default()),
        );
        let mut result = crate::domain::project::types::Projected::default();
        assert!(matches!(
            service.consulted(&[FilePath("required".into())], &mut result),
            Err(FileAnalysisError::Corpus(_))
        ));
        assert!(result.consulted.is_empty() && result.consulted_absent.is_empty());
    }
    let service = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::new(LanguageFake::default()),
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let mut result = crate::domain::project::types::Projected::default();
    service
        .consulted(&[FilePath("absent".into())], &mut result)
        .expect("a genuine missing dependency is recordable");
    assert_eq!(result.consulted_absent, vec![FilePath("absent".into())]);
}

/// A legal unaudited worker whose immutable resident alternatives change between reads.
struct UnstableResidentFacts {
    /// Delegate mutation/retained-word operations without promising resident identity.
    inner: Box<dyn crate::outbound::facts::interface::FactsWorker>,
    /// Distinguishable same-shape populations selected by read order.
    alternatives: [Vec<ryo_language::domain::relations::types::FactColumn>; 2],
    /// Source reads remain observable even when their shapes agree.
    reads: std::sync::atomic::AtomicUsize,
}
impl crate::outbound::facts::interface::FactsWorker for UnstableResidentFacts {
    fn resident_stability(&self) -> crate::outbound::facts::types::ResidentStability {
        crate::outbound::facts::types::ResidentStability::Unstable
    }
    fn columns_mut(
        &mut self,
    ) -> Result<
        crate::outbound::facts::types::ColumnsMut<'_>,
        crate::outbound::facts::errors::FactsError,
    > {
        self.inner.columns_mut()
    }
    fn rows(
        &self,
        relation: Relation,
    ) -> Result<
        ryo_language::domain::relations::types::ColumnsView<'_>,
        crate::outbound::facts::errors::FactsError,
    > {
        use ryo_language::domain::relations::{service::kinds, types::ColumnsView};
        if relation != Relation::Function {
            return self.inner.rows(relation);
        }
        let place = self.reads.fetch_add(1, Ordering::Relaxed) % 2;
        let columns = self
            .alternatives
            .get(place)
            .expect("one of two immutable alternatives");
        Ok(ColumnsView {
            kinds: kinds(relation),
            columns,
        })
    }
    fn keep(
        &mut self,
        bucket: ryo_values::domain::stored_facts::types::Bucket,
        words: crate::outbound::facts::types::WordsToKeep<'_>,
    ) -> Result<(), crate::outbound::facts::errors::FactsError> {
        self.inner.keep(bucket, words)
    }
    fn release(&mut self) -> Result<(), crate::outbound::facts::errors::FactsError> {
        self.inner.release()
    }
    fn finished(
        self: Box<Self>,
    ) -> Result<
        Box<dyn crate::outbound::facts::interface::FactsFinish>,
        crate::outbound::facts::errors::FactsError,
    > {
        self.inner.finished()
    }
}

fn prepared_resource_state(
    enabled: bool,
    partial: &[crate::domain::coverage::types::Partial],
) -> (bool, bool) {
    use crate::application::file_dag::types::FileDagExecution;
    use crate::domain::file_program::types::RequestGraph;
    use crate::domain::operator::types::PreparedPool;
    let graph = RequestGraph {
        derived_sources: Vec::new(),
        plans: Vec::new(),
        nodes: Vec::new(),
        cuts: Vec::new(),
        consumers: Vec::new(),
        boundaries: Vec::new(),
        rule_liveness: Vec::new(),
    };
    let dag = FileDagExecution::new_for_population(Arc::new(graph), |_| true, &[], true);
    dag.prepare_resources(
        enabled,
        &[Relation::Function],
        partial,
        &[Relation::Function],
        &[],
        &PreparedPool::default(),
        |_| true,
    );
    dag.resource_eligibility(Relation::Function)
}

#[test]
fn physical_missing_kinds_remain_ineligible_when_rows_mode_clears_evaluation_coverage() {
    use crate::domain::coverage::types::{Lacking, LackingKind, Partial};
    use ryo_values::domain::stored_facts::types::{LanguageNumber, PartialCause};
    let mut tables = tables_for(
        WorkerNumber(0),
        &[],
        TreeSteps::AsOperators,
        Counting::DoesNotCount,
    );
    tables.language = Some(LanguageNumber(0));
    tables.partials.push(Partial {
        relation: Relation::Function,
        cause: PartialCause("missing one physical kind".into()),
        lacking: Lacking::Kind(LackingKind("async".into())),
        at: None,
    });

    let facts = FactsFake::default().worker().expect("facts");
    let enabled = FileAnalysis::resource_generation_enabled(&tables, facts.as_ref());
    let findings = prepared_resource_state(enabled, &tables.partials);
    let rows = prepared_resource_state(enabled, &tables.partials);
    assert_eq!(findings, rows);
    assert_eq!(
        rows,
        (true, false),
        "partial physical rows cannot be eligible"
    );
    tables.language = None;
    assert_eq!(
        prepared_resource_state(
            FileAnalysis::resource_generation_enabled(&tables, facts.as_ref()),
            &[]
        ),
        (false, false)
    );
}

#[test]
fn unstable_same_shape_resident_populations_decline_generation_reuse() {
    use crate::outbound::facts::interface::FactsWorker as _;
    use ryo_language::domain::relations::types::FactColumn;
    use ryo_values::domain::stored_facts::types::LanguageNumber;
    let mut tables = tables_for(
        WorkerNumber(0),
        &[],
        TreeSteps::AsOperators,
        Counting::DoesNotCount,
    );
    tables.language = Some(LanguageNumber(0));
    let facts = UnstableResidentFacts {
        inner: FactsFake::default().worker().expect("inner"),
        alternatives: [
            vec![
                FactColumn(vec![1]),
                FactColumn(vec![0]),
                FactColumn(vec![0]),
                FactColumn(vec![4]),
            ],
            vec![
                FactColumn(vec![2]),
                FactColumn(vec![0]),
                FactColumn(vec![0]),
                FactColumn(vec![4]),
            ],
        ],
        reads: std::sync::atomic::AtomicUsize::new(0),
    };
    let first = facts
        .rows(Relation::Function)
        .expect("first resident population")
        .columns
        .first()
        .expect("first column")
        .0
        .clone();
    let enabled = FileAnalysis::resource_generation_enabled(&tables, &facts);
    assert_eq!(prepared_resource_state(enabled, &[]), (false, false));
    let second = facts
        .rows(Relation::Function)
        .expect("changed population")
        .columns
        .first()
        .expect("first column")
        .0
        .clone();
    assert_ne!(
        first, second,
        "same relation/schema/count is insufficient without an audited stability promise"
    );
}

#[test]
fn actual_file_visit_admits_only_stable_resource_generation() {
    use crate::application::file_dag::resources::take_resource_setup;
    use crate::domain::render::types::Answering;
    use crate::domain::visit::types::Keeping;
    use ryo_language::domain::relations::types::FactColumn;
    let _previous = take_resource_setup();
    let (stable, _) = visited_as(
        &["stable.rs"],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Discards,
        Arc::new(DigestFake),
    )
    .expect("stable file visit");
    assert!(stable.failure.is_none());
    assert_eq!(take_resource_setup(), Some((true, true)));

    let alternate = |value| (0..4).map(|_| FactColumn(vec![value])).collect();
    let facts = UnstableResidentFacts {
        inner: FactsFake::default().worker().expect("facts"),
        alternatives: [alternate(0), alternate(1)],
        reads: std::sync::atomic::AtomicUsize::new(0),
    };
    let (unstable, _) = visited_as_with_facts(
        &["unstable.rs"],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Discards,
        Arc::new(DigestFake),
        Box::new(facts),
    )
    .expect("unstable file visit");
    assert!(unstable.failure.is_none());
    assert_eq!(take_resource_setup(), Some((false, false)));
}

#[test]
fn a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket() {
    use super::types::{FileVisit, Worker};
    use crate::domain::table_model::types::{RowCount, Table};
    use crate::domain::table_words::service::{encode_table, pack};
    use crate::domain::table_words::types::WordSlice;
    use crate::domain::visit::types::{Keeping, Noting, RoleRead};
    use crate::outbound::digest::fixtures::fake::DigestCounting;
    use ryo_language::domain::relations::types::{
        CellKind, FileId, Integer, Name, NamePlace, Offset, Origin, PlaceNumber, TextHash, Value,
        Words,
    };
    use ryo_values::domain::records::types::Kept;
    use ryo_values::domain::stored_facts::types::{ArenaBase, Bucket, KeptWords};
    let plan = Plan {
        parameters: vec![],
        steps: vec![],
        comparison_origins: vec![],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![ryo_values::domain::plan_parts::types::Input::Step(StepId(
            0,
        ))],
    };
    let plans = [plan];
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: crate::outbound::matching::fixtures::fake::MatchingFake
            .compile(&[])
            .expect("matcher"),
    };
    let visit = FileVisit {
        store: None,
        setting: VisitSetting {
            source_language: None,
            executor: &crate::application::file_dag::execution::CommonOperatorEngine,
            operators: crate::application::file_dag::testing::request_fixture(&plans, &[], &[]),
            domain: crate::domain::workers::types::VisitSetting {
                plans: &plans,
                bindings: &[],
                wanted: &[],
                paths: &[],
                first_bucket: &[Bucket(0)],
                ids: &[],
                applicability: &[],
                applicability_can_gate: true,
                placed: &placed,
                records: &[],
                replaying: crate::domain::workers::types::Replaying::Findings,
                stored: &NOTHING_STORED,
                handed: &NOTHING_HANDED,
                wanted_buckets: &NOTHING_WANTED,
                budget: ryo_values::domain::runs::types::MemoryBytes(u64::MAX),
                noted: Noting::Unwanted,
                roled: RoleRead::Skips,
                needs: &[],
                answering: &crate::domain::render::types::Answering::Findings,
                keeping: Keeping::Keeps,
                unknown: &[],
            },
        },
    };
    let digest = Arc::new(DigestCounting::default());
    let analysis = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::new(LanguageFake::default()),
        digest.clone(),
        Arc::new(MemoryFake::default()),
    );
    let mut worker = Worker {
        tables: tables_for(
            WorkerNumber(0),
            &plans,
            TreeSteps::Lowered,
            Counting::DoesNotCount,
        ),
        facts: FactsFake::default().worker().expect("facts worker"),
        old_rows: None,
        failure: None,
    };
    let packed = |value, places: &[NamePlace]| {
        let mut words = Words::default();
        let kind = if matches!(value, Value::File(_)) {
            CellKind::File
        } else if matches!(value, Value::Name(_)) {
            CellKind::Symbol
        } else {
            CellKind::Number
        };
        encode_table(
            &Table {
                kinds: vec![kind],
                cells: vec![value],
                count: RowCount(1),
            },
            places,
            &mut words,
        );
        KeptWords(pack(WordSlice(&words.0)))
    };
    let good = Kept {
        bucket: Bucket(0),
        words: packed(Value::Number(Integer(7)), &[]),
    };
    for (value, places) in [
        (Value::File(FileId(99)), vec![]),
        (
            Value::Name(Name {
                hash: TextHash(1),
                place: PlaceNumber(0),
            }),
            vec![NamePlace {
                origin: Origin::Derived(WorkerNumber(99)),
                file: FileId(0),
                start: Offset(9),
                end: Offset(12),
            }],
        ),
        (
            Value::Name(Name {
                hash: TextHash(1),
                place: PlaceNumber(0),
            }),
            vec![NamePlace {
                origin: Origin::Derived(WorkerNumber(0)),
                file: FileId(0),
                start: Offset(2),
                end: Offset(4),
            }],
        ),
    ] {
        worker.tables.captured = vec![
            good.clone(),
            Kept {
                bucket: Bucket(1),
                words: packed(value, &places),
            },
        ];
        assert!(
            analysis
                .recorded(&worker, &visit, FileId(0), ArenaBase(7))
                .is_none()
        );
        assert_eq!(
            digest.digests.load(Ordering::Relaxed),
            0,
            "no digest before every captured remap succeeds"
        );
    }
    worker.tables.captured = vec![good];
    assert!(
        analysis
            .recorded(&worker, &visit, FileId(0), ArenaBase(7))
            .is_some()
    );
    assert_eq!(
        digest.digests.load(Ordering::Relaxed),
        2,
        "nonvacuous rule-flat then noted digest control"
    );
}

/// Only post-normalization outcomes can reach the staging boundary; IO failures remain failures.
#[test]
fn successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads() {
    use crate::outbound::lint_store::errors::LintStoreError;
    use crate::outbound::lint_store::interface::ReadStore;
    use ryo_values::domain::files::types::FileNumber;
    use ryo_values::domain::records::types::{
        FileOutcome, FindingRows, Handed, HandedWords, Handing, Ran, RecordWords, StagedWords,
        StagingSession,
    };
    use ryo_values::domain::stored_facts::types::{Bucket, Digest, PackedWords};
    use ryo_values::domain::stores::types::{BucketsWanted, KeptRows, Stored};
    use std::sync::atomic::AtomicUsize;

    #[derive(Debug, Default)]
    struct StagingRead {
        calls: AtomicUsize,
        fail: bool,
    }
    impl ReadStore for StagingRead {
        fn tables(&self) -> Stored {
            Stored::default()
        }
        fn handed(
            &self,
            _: &BucketsWanted,
        ) -> Result<ryo_values::domain::stores::types::ReadHanded, LintStoreError> {
            Ok(ryo_values::domain::stores::types::ReadHanded::default())
        }
        fn keep(&self, _: Stored, _: &KeptRows) -> Result<(), LintStoreError> {
            Ok(())
        }
        fn stage_words(&self, words: PackedWords) -> Result<RecordWords, LintStoreError> {
            let offset = self.calls.fetch_add(1, Ordering::Relaxed);
            if self.fail {
                return Err(LintStoreError::NothingRead);
            }
            Ok(RecordWords::Staged(StagedWords {
                session: StagingSession(1),
                offset: u64::try_from(offset).expect("small count"),
                length: u64::try_from(words.0.len()).expect("size"),
            }))
        }
    }
    let outcome = FileOutcome::Read(Ran {
        rows: FindingRows::default(),
        number: FileNumber(0),
        interface: Digest([1; 16]),
        noted: Digest([2; 16]),
        frontier: vec![],
        handed: Handing::Made(Handed {
            buckets: (0..64)
                .map(|bucket| HandedWords {
                    bucket: Bucket(bucket),
                    words: PackedWords(vec![7; 4096]).into(),
                })
                .collect(),
            ..Handed::default()
        }),
    });
    let read = StagingRead::default();
    assert!(
        FileAnalysis::stage_outcome(Some(&read), None)
            .expect("no outcome")
            .is_none()
    );
    assert_eq!(read.calls.load(Ordering::Relaxed), 0);
    assert_eq!(
        FileAnalysis::stage_outcome(None, Some(outcome.clone())).expect("no store"),
        Some(outcome.clone())
    );
    let Some(FileOutcome::Read(ran)) =
        FileAnalysis::stage_outcome(Some(&read), Some(outcome.clone())).expect("stage")
    else {
        panic!("read")
    };
    assert_eq!(ran.interface, Digest([1; 16]));
    assert_eq!(ran.noted, Digest([2; 16]));
    let Handing::Made(handed) = ran.handed else {
        panic!("fresh outcome")
    };
    assert_eq!(read.calls.load(Ordering::Relaxed), 64);
    assert!(handed.buckets.iter().all(|row| matches!(
        row.words,
        RecordWords::Staged(StagedWords { length: 4096, .. })
    )));
    let failing = StagingRead {
        fail: true,
        ..StagingRead::default()
    };
    assert!(matches!(
        FileAnalysis::stage_outcome(Some(&failing), Some(outcome)),
        Err(FileAnalysisError::Store(LintStoreError::NothingRead))
    ));
    assert_eq!(failing.calls.load(Ordering::Relaxed), 1);
}

#[test]
fn staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops() {
    use super::types::{FileVisit, Worker};
    use crate::domain::table_model::types::{RowCount, Table};
    use crate::domain::table_words::service::{encode_table, pack};
    use crate::domain::table_words::types::WordSlice;
    use crate::domain::visit::types::{Keeping, Noting, RoleRead};
    use crate::domain::workers::types::FileOrder;
    use crate::outbound::lint_store::interface::LintStorePort as _;
    use crate::outbound::lint_store::types::{BuildIdentity, CacheDirectory, Store};
    use ryo_language::domain::relations::types::{CellKind, FileId, Integer, Value, Words};
    use ryo_values::domain::files::types::{FileNumber, FileSize, Nanoseconds};
    use ryo_values::domain::records::types::{
        FileOutcome, FindingRows, Handed, HandedWords, Handing, Ran, Record, RecordWords,
        StagedWords, StagingSession,
    };
    use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket, Digest};
    use ryo_values::domain::stores::types::BucketsWanted;

    let directory = tempfile::tempdir().expect("snapshot directory");
    let store = crate::outbound::lint_store::factory::postcard(Store {
        directory: CacheDirectory(directory.path().to_string_lossy().into()),
        build: BuildIdentity("replay-staging".into()),
    });
    let snapshot = store.stored().expect("read snapshot");
    let new_worker = || Worker {
        tables: tables_for(
            WorkerNumber(0),
            &[],
            TreeSteps::Lowered,
            Counting::DoesNotCount,
        ),
        facts: FactsFake::default().worker().expect("worker"),
        old_rows: None,
        failure: None,
    };
    let producer = new_worker();
    let mut words = Words::default();
    encode_table(
        &Table {
            kinds: vec![CellKind::Number],
            cells: vec![Value::Number(Integer(7))],
            count: RowCount(1),
        },
        &[],
        &mut words,
    );
    let packed = pack(WordSlice(&words.0));
    let ran = Ran {
        rows: FindingRows::default(),
        number: FileNumber(0),
        interface: Digest([1; 16]),
        noted: Digest([2; 16]),
        frontier: vec![],
        handed: Handing::Made(Handed {
            buckets: vec![HandedWords {
                bucket: Bucket(0),
                words: packed.into(),
            }],
            text: ArenaBytes::default(),
            ..Handed::default()
        }),
    };
    let outcome =
        FileAnalysis::stage_outcome(Some(snapshot.as_ref()), Some(FileOutcome::Read(ran)))
            .expect("normalized outcome staged")
            .expect("outcome");
    drop(producer);
    let mut record = Record {
        path: FilePath("case.py".into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
        digest: Digest([3; 16]),
        number: FileNumber(0),
        outcome,
    };
    let FileOutcome::Read(ran) = &mut record.outcome else {
        panic!("read")
    };
    let Handing::Made(handed) = &mut ran.handed else {
        panic!("fresh")
    };
    handed.buckets.push(HandedWords {
        bucket: Bucket(1),
        words: RecordWords::Staged(StagedWords {
            session: StagingSession(u64::MAX),
            offset: u64::MAX,
            length: 9,
        }),
    });
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: crate::outbound::matching::fixtures::fake::MatchingFake
            .compile(&[])
            .expect("matcher"),
    };
    let wanted = BucketsWanted::These(vec![Bucket(0)]);
    let visit = FileVisit {
        store: Some(snapshot.as_ref()),
        setting: VisitSetting {
            source_language: None,
            executor: &crate::application::file_dag::execution::CommonOperatorEngine,
            operators: crate::application::file_dag::testing::request_fixture(&[], &[], &[]),
            domain: crate::domain::workers::types::VisitSetting {
                plans: &[],
                bindings: &[],
                wanted: &[],
                paths: &[],
                first_bucket: &[],
                ids: &[],
                applicability: &[],
                applicability_can_gate: true,
                placed: &placed,
                records: &[],
                replaying: crate::domain::workers::types::Replaying::FindingsAndHanded,
                stored: &NOTHING_STORED,
                handed: &NOTHING_HANDED,
                wanted_buckets: &wanted,
                budget: ryo_values::domain::runs::types::MemoryBytes(u64::MAX),
                noted: Noting::Unwanted,
                roled: RoleRead::Skips,
                needs: &[],
                answering: &crate::domain::render::types::Answering::Findings,
                keeping: Keeping::Keeps,
                unknown: &[],
            },
        },
    };
    let mut replay = new_worker();
    FileAnalysis::hand_again(&mut replay, &visit, FileOrder(0), FileId(0), &record)
        .expect("invalid unrequested span is not read");
    let all_visit = FileVisit {
        setting: VisitSetting {
            domain: crate::domain::workers::types::VisitSetting {
                wanted_buckets: &BucketsWanted::All,
                ..(visit.setting.clone()).domain
            },
            ..visit.setting.clone()
        },
        store: visit.store,
    };
    assert!(
        FileAnalysis::hand_again(&mut replay, &all_visit, FileOrder(0), FileId(0), &record)
            .is_err()
    );

    // The persistent old-row path uses the same filter-before-resolution contract.
    use ryo_values::domain::stores::types::{
        KeptRows, OldWords, ReadWordPayload, ReadWords, StoreSnapshot, StoredFileNumber,
    };
    let mut valid = ryo_values::domain::stored_facts::types::PackedWords::default();
    let FileOutcome::Read(ran) = &record.outcome else {
        panic!("read")
    };
    let Handing::Made(handed) = &ran.handed else {
        panic!("made")
    };
    let RecordWords::Staged(descriptor) = &handed.buckets[0].words else {
        panic!("staged")
    };
    snapshot
        .read_staged(descriptor, &mut valid)
        .expect("valid packed words");
    let mut persisted = crate::outbound::lint_store::fixtures::fake::stored(1);
    persisted.files[0].path = record.path.clone();
    persisted.handed.frontier[0].bucket = Bucket(0);
    persisted.handed.frontier[0].words = valid;
    snapshot
        .keep(persisted.clone(), &KeptRows::default())
        .expect("persist old fixture");
    let old = store.stored().expect("old snapshot");
    let mut old_handed = old.handed(&BucketsWanted::All).expect("headers only");
    let invalid = OldWords {
        snapshot: StoreSnapshot(u64::MAX),
        bucket: Bucket(1),
        file: StoredFileNumber(0),
        header: u64::MAX,
        offset: u64::MAX,
        length: 9,
    };
    old_handed.frontier.push(ReadWords {
        file: StoredFileNumber(0),
        bucket: Bucket(1),
        words: ReadWordPayload::Old(invalid),
    });
    old_handed.frontier.push(ReadWords {
        file: StoredFileNumber(1),
        bucket: Bucket(0),
        words: ReadWordPayload::Old(invalid),
    });
    let mut old_record = record.clone();
    let FileOutcome::Read(ran) = &mut old_record.outcome else {
        panic!("read")
    };
    ran.handed = Handing::Kept;
    let old_visit = FileVisit {
        store: Some(old.as_ref()),
        setting: VisitSetting {
            domain: crate::domain::workers::types::VisitSetting {
                stored: &persisted,
                handed: &old_handed,
                ..(visit.setting.clone()).domain
            },
            ..visit.setting.clone()
        },
    };
    FileAnalysis::hand_again(
        &mut replay,
        &old_visit,
        FileOrder(0),
        FileId(0),
        &old_record,
    )
    .expect("unrequested bucket and other file never resolve their invalid descriptors");
    let old_all = FileVisit {
        store: old_visit.store,
        setting: VisitSetting {
            domain: crate::domain::workers::types::VisitSetting {
                wanted_buckets: &BucketsWanted::All,
                ..(old_visit.setting).domain
            },
            ..old_visit.setting
        },
    };
    assert!(
        FileAnalysis::hand_again(&mut replay, &old_all, FileOrder(0), FileId(0), &old_record)
            .is_err()
    );
}
