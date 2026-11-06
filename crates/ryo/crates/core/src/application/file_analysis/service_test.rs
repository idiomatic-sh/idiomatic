//! Analysis uses its injected language and propagates required failures.
use super::errors::FileAnalysisError;
use super::service::{FileAnalysis, recovery_of};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::digest::fixtures::fake::DigestFake;
use crate::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution::inbound::use_cases::execution::types::facts::FactsPort as _;

use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::language::types::{ProjectToRead, ProjectNotes};
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_language::inbound::use_cases::project::types::{ManifestFile, ManifestText};
use std::sync::{Arc, atomic::Ordering};

use super::types::RequestVisitSetting;
use crate::domain::workers::service::tables_for;
use ryo_execution::inbound::use_cases::execution::types::Counting;
use ryo_execution::inbound::use_cases::execution::types::matching::MatchingPort as _;
use ryo_execution::inbound::use_cases::execution::{
    interface::ExecutionUseCase,
    types::{ObservationIdentity, PreparedRequest},
    use_case::ExecutionUseCaseImpl,
};
use ryo_execution::inbound::use_cases::metadata::interface::{Mask, PlacedScopes, ScopePlacement};
use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_relations::domain::relations::types::{Relation, WorkerNumber};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::{Emit, Plan};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};

fn prepared_test_program(
    plans: &[Plan],
    ids: &[RuleId],
    corpus: &[ryo_values::domain::files::types::CorpusInput],
) -> (
    Arc<ryo_execution::inbound::use_cases::execution::types::PreparedExecutionProgram>,
    ryo_execution::inbound::use_cases::execution::types::RequestExecution,
) {
    use ryo_values::domain::{
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        stored_facts::types::Digest,
    };
    let engine = ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    let program = Arc::new(
        engine
            .prepare(PreparedRequest {
                plans: plans.to_vec(),
                bindings: vec![Vec::new(); plans.len()],
                rules: ids
                    .iter()
                    .map(|id| RuleSpec {
                        id: id.clone(),
                        query: QueryBundle {
                            root: QueryDocumentSource {
                                id: QuerySourceId(id.0.clone()),
                                text: "".into(),
                            },
                            modules: Default::default(),
                        },
                        applicability: RuleApplicability::Always,
                        given: Vec::new(),
                        observations: Vec::new(),
                        spelled: RuleParameters("".into()),
                        include: Vec::new(),
                        exclude: Vec::new(),
                    })
                    .collect(),
                digests: vec![Digest([0; 16]); plans.len()],
                hosted: Vec::new(),
                observations: ObservationIdentity::Independent,
            })
            .expect("valid prepared test request"),
    );
    let request = engine.begin(&program, corpus);
    (program, request)
}

/// A plan of one rule that captures one frontier input.
fn one_capture_plan() -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![ryo_values::domain::plans::types::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
            operation: ryo_values::domain::plans::types::Operation::Select {
                from: ryo_values::domain::plan_parts::types::Input::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(0)],
            },
        }],
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
    }
}

use ryo_values::domain::stores::types::BucketsWanted;

/// Replays one handed bucket, kept under `proof`, into a worker of the program of `one_capture_plan`, when the run wants `wanted`.
fn replay_under(
    proof: ryo_values::domain::records::types::FrontierProof,
    wanted: &BucketsWanted,
) -> Result<(), FileAnalysisError> {
    replay_under_capture(proof, wanted, true)
}

/// As `replay_under`, with the prepared root's capture flag of the bucket's frontier set to `captured`.
fn replay_under_capture(
    proof: ryo_values::domain::records::types::FrontierProof,
    wanted: &BucketsWanted,
    captured: bool,
) -> Result<(), FileAnalysisError> {
    use super::types::{FileVisit, Worker};
    use crate::domain::visit::types::{Keeping, Noting, RoleRead};
    use crate::domain::workers::types::FileOrder;
    use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value, Words};
    use ryo_values::domain::files::types::{FileNumber, FileSize, Nanoseconds};
    use ryo_values::domain::records::types::{
        FileOutcome, Handed, HandedWords, Handing, Ran, Record,
    };
    use ryo_values::domain::row_texts::types::FindingRows;
    use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket, Digest};
    use ryo_values::domain::table_words::service::{encode_table, pack};
    use ryo_values::domain::table_words::types::WordSlice;
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
    let record = Record {
        executable: Default::default(),
        path: FilePath("case.py".into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
        digest: Digest([3; 16]),
        number: FileNumber(0),
        outcome: FileOutcome::Read(Ran {
            rows: FindingRows::default(),
            number: FileNumber(0),
            interface: Digest([1; 16]),
            noted: Digest([2; 16]),
            frontier: vec![],
            handed: Handing::Made(Handed {
                buckets: vec![HandedWords {
                    bucket: Bucket(0),
                    proof,
                    words: pack(WordSlice(&words.0)).into(),
                }],
                text: ArenaBytes::default(),
                ..Handed::default()
            }),
        }),
    };
    let plans = [one_capture_plan()];
    let ids = [RuleId("capture".into())];
    let mut metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    for rule in &mut metadata {
        rule.frontier_capture.fill(if captured {
            ryo_values::domain::run_metadata::types::FrontierFeed::Captured
        } else {
            ryo_values::domain::run_metadata::types::FrontierFeed::CrossFile
        });
    }
    let (program, request) = prepared_test_program(&plans, &ids, &[]);
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: crate::outbound::matching::fixtures::fake::MatchingFake
            .compile(&[])
            .expect("matcher"),
    };
    let visit = FileVisit {
        store: None,
        setting: RequestVisitSetting {
            source_language: None,
            program,
            services: &request,
            domain: crate::domain::workers::types::VisitSetting {
                metadata: &metadata,
                wanted: &[],
                paths: &[],
                first_bucket: &[Bucket(0)],
                ids: &ids,
                applicability: &[],
                applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
                placed: &placed,
                records: &[],
                reuse: &ryo_values::domain::stores::types::Reuse::Cross(
                    ryo_values::domain::stores::types::MadeAgain::All,
                ),
                stored: &NOTHING_STORED,
                handed: &NOTHING_HANDED,
                wanted_buckets: wanted,
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
    let mut worker = Worker {
        tables: tables_for(WorkerNumber(0), &[]),
        facts: FactsFake::default().worker().expect("worker"),
        old_rows: None,
        failure: None,
    };
    FileAnalysis::hand_again(&mut worker, &visit, FileOrder(0), FileId(0), &record)
}

#[test]
fn a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read() {
    use ryo_values::domain::records::types::FrontierCoverage;
    use ryo_values::domain::stored_facts::types::{Bucket, Digest};
    let good = crate::outbound::lint_store::fixtures::proof();
    let selected = BucketsWanted::These(vec![Bucket(0)]);
    replay_under(good.clone(), &selected).expect("the proof of the prepared root is accepted");
    let mut wrong_digest = good.clone();
    wrong_digest.rule_digest = Digest([9; 16]);
    let mut wrong_ordinal = good.clone();
    wrong_ordinal.input = ryo_values::domain::records::types::FrontierOrdinal(1);
    let mut wrong_coverage = good.clone();
    wrong_coverage.coverage = FrontierCoverage::Partial(Vec::new());
    let refused = |result: Result<(), FileAnalysisError>| {
        matches!(
            result,
            Err(FileAnalysisError::Execution(
                ryo_execution::inbound::use_cases::execution::types::OperatingError::ObservationIncomplete { .. }
            ))
        )
    };
    assert!(
        refused(replay_under_capture(good.clone(), &selected, false)),
        "a bucket whose frontier the prepared root does not capture is refused when selected"
    );
    replay_under_capture(good.clone(), &BucketsWanted::These(Vec::new()), false)
        .expect("the capture flag of a bucket the run does not select is never checked");
    for stale in [wrong_digest, wrong_ordinal, wrong_coverage] {
        assert!(
            matches!(
                replay_under(stale.clone(), &selected),
                Err(FileAnalysisError::Execution(
                    ryo_execution::inbound::use_cases::execution::types::OperatingError::ObservationIncomplete { .. }
                ))
            ),
            "{stale:?} must be refused for a selected bucket"
        );
        replay_under(stale, &BucketsWanted::These(Vec::new()))
            .expect("the proof of a bucket the run does not select is never checked");
    }
}

#[test]
fn syntax_applicability_skips_only_gated_rules_and_names_the_reason() {
    let plan = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![ryo_values::domain::plans::types::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
            operation: ryo_values::domain::plans::types::Operation::Select {
                from: ryo_values::domain::plan_parts::types::Input::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(0)],
            },
        }],
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
    let metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    let (program, request) = prepared_test_program(&plans, &ids, &[]);
    let setting = RequestVisitSetting {
        source_language: None,
        program,
        services: &request,
        domain: crate::domain::workers::types::VisitSetting {
            metadata: &metadata,
            wanted: &[Relation::Function, Relation::Identifier],
            paths: &paths,
            first_bucket: &[],
            ids: &ids,
            applicability: &applicability,
            applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
            placed: &placed,
            records: &[],
            reuse: &ryo_values::domain::stores::types::Reuse::Whole,
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
    setting: &RequestVisitSetting<'_>,
    acceptance: SyntaxAcceptance,
    expected_reason: &str,
) {
    let mut tables = tables_for(WorkerNumber(0), setting.metadata);
    tables.mask = Mask(vec![0b11]);
    let skipped = FileAnalysis::gate_rules(
        &mut tables,
        setting,
        ryo_relations::domain::relations::types::FileId(0),
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
fn assert_accepted_and_missing_applicability(setting: &RequestVisitSetting<'_>) {
    let mut tables = tables_for(WorkerNumber(0), setting.metadata);
    tables.mask = Mask(vec![0b11]);
    assert_eq!(
        FileAnalysis::gate_rules(
            &mut tables,
            setting,
            ryo_relations::domain::relations::types::FileId(0),
            SyntaxAcceptance::Accepted,
        ),
        [false, false]
    );
    assert_eq!(tables.mask.0, [0b11]);
    assert_eq!(
        FileAnalysis::active_wanted(&tables, setting, SyntaxAcceptance::Accepted).as_ref(),
        [Relation::Function, Relation::Identifier]
    );
    let always_setting = RequestVisitSetting {
        source_language: None,
        domain: crate::domain::workers::types::VisitSetting {
            metadata: &setting.metadata[1..],
            ids: &setting.ids[1..],
            applicability: &setting.applicability[1..],
            applicability_can_gate: crate::domain::workers::types::Gating::NeverGates,
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

    let missing_policy = RequestVisitSetting {
        domain: crate::domain::workers::types::VisitSetting {
            applicability: &[],
            applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
            ..(setting.clone()).domain
        },
        ..setting.clone()
    };
    assert!(FileAnalysis::may_gate_rules(
        &missing_policy,
        SyntaxAcceptance::Accepted
    ));
    let mut missing_tables = tables_for(WorkerNumber(0), setting.metadata);
    missing_tables.mask = Mask(vec![0b11]);
    assert_eq!(
        FileAnalysis::gate_rules(
            &mut missing_tables,
            &missing_policy,
            ryo_relations::domain::relations::types::FileId(0),
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
    use crate::outbound::language::types::FileReading;
    use ryo_execution::inbound::use_cases::metadata::interface::Recovery;
    use ryo_language::domain::elements::service::{diagnostics, has_incomplete_lexical_stream, has_incomplete_syntax};
    use ryo_language::domain::builders::service::{diagnose, diagnose_with_metadata, table_builder};
    use ryo_language::domain::elements::types::Elements;
    use ryo_language::domain::diagnostics::types::{SyntaxCompleteness, DiagnosticCode, DiagnosticMetadata, DiagnosticPhase, TreeImpact};
    use ryo_language::domain::symbols::types::ByteSlice;
    use ryo_relations::domain::relations::types::{Offset, ReportText};

    let source = ByteSlice(b"x");
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    {
        let mut builder = table_builder(&mut elements, &mut symbols, source);
        diagnose_with_metadata(
            &mut builder,
            DiagnosticMetadata { phase: DiagnosticPhase::Validation, impact: TreeImpact::Preserved, code: DiagnosticCode("fixture.target-syntax") },
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
    assert_eq!(has_incomplete_syntax(&elements), SyntaxCompleteness::Complete);
    assert_eq!(
        diagnostics(&elements).count(),
        1,
        "diagnostic remains reportable"
    );
    assert!(
        MetadataUseCaseImpl
            .partial_of(&[], preserved, &[Relation::Function])
            .is_empty()
    );

    let mut lexical_elements = Elements::default();
    let mut lexical_symbols = Symbols::default();
    {
        let mut lexical_builder =
            table_builder(&mut lexical_elements, &mut lexical_symbols, source);
        diagnose_with_metadata(
            &mut lexical_builder,
            DiagnosticMetadata { phase: DiagnosticPhase::Parse, impact: TreeImpact::LexicallyComplete, code: DiagnosticCode("fixture.parser-recovered") },
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
    assert_eq!(has_incomplete_syntax(&lexical_elements), SyntaxCompleteness::Incomplete);
    assert_eq!(has_incomplete_lexical_stream(&lexical_elements), SyntaxCompleteness::Complete);
    let lexical_partials = MetadataUseCaseImpl.partial_of(
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
    assert_eq!(has_incomplete_syntax(&incomplete_elements), SyntaxCompleteness::Incomplete);
    assert_eq!(has_incomplete_lexical_stream(&incomplete_elements), SyntaxCompleteness::Incomplete);
    assert_eq!(diagnostics(&incomplete_elements).count(), 1);
    assert!(
        !MetadataUseCaseImpl
            .partial_of(&[], incomplete, &[Relation::Function])
            .is_empty()
    );
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
    let input = ProjectToRead {
        search_scope: &Default::default(),
        given_inventory: &Default::default(),
        observed_entries: &Default::default(),
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
            limit: ryo_values::domain::row_texts::types::RowTotal(0),
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
    digest: Arc<dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort>,
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
    digest: Arc<dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort>,
    facts: Box<dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker>,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    use crate::outbound::corpus::interface::CorpusPort as _;
    use ryo_values::domain::files::types::{CorpusFile, FileBytes};
    let corpus = Arc::new(CorpusFake {
        files: paths
            .iter()
            .map(|path| CorpusFile {
                executable: Default::default(),
                path: FilePath((*path).into()),
                bytes: FileBytes(b"fn main() {}".to_vec()),
            })
            .collect(),
        ..CorpusFake::default()
    });
    let files = corpus.inventory().expect("fake corpus inventory");
    visited_inputs(
        corpus,
        &files.0,
        &[Relation::Function],
        language,
        answering,
        keeping,
        digest,
        facts,
        None,
        1,
    )
}

/// Runs real file admission against injected input observations, retaining the worker for semantic assertions.
fn visited_inputs(
    corpus: Arc<dyn crate::outbound::corpus::interface::CorpusPort>,
    files: &[ryo_values::domain::files::types::CorpusInput],
    wanted: &[Relation],
    language: LanguageFake,
    answering: &crate::domain::render::types::Answering,
    keeping: crate::domain::visit::types::Keeping,
    digest: Arc<dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort>,
    facts: Box<dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker>,
    kept: Option<&ryo_values::domain::records::types::Record>,
    repetitions: usize,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    visited_inputs_with_rule_demands(
        corpus,
        files,
        wanted,
        language,
        answering,
        keeping,
        digest,
        facts,
        kept,
        repetitions,
        &[],
        &[],
        Mask(vec![1]),
    )
}

fn visited_inputs_with_rule_demands(
    corpus: Arc<dyn crate::outbound::corpus::interface::CorpusPort>,
    files: &[ryo_values::domain::files::types::CorpusInput],
    wanted: &[Relation],
    language: LanguageFake,
    answering: &crate::domain::render::types::Answering,
    keeping: crate::domain::visit::types::Keeping,
    digest: Arc<dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort>,
    facts: Box<dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker>,
    kept: Option<&ryo_values::domain::records::types::Record>,
    repetitions: usize,
    ids: &[ryo_values::domain::runs::types::RuleId],
    needs: &[ryo_execution::inbound::use_cases::metadata::interface::Needs],
    always: Mask,
) -> Result<(super::types::Worker, Arc<LanguageFake>), Failure> {
    use super::types::{FileVisit, Worker};
    use crate::domain::visit::types::{Noting, RoleRead};
    use crate::outbound::matching::fixtures::fake::MatchingFake;
    use crate::outbound::scheduler::types::JobNumber;
    use ryo_values::domain::runs::types::MemoryBytes;
    let language = Arc::new(language);
    let service = FileAnalysis::new(
        corpus,
        Arc::<LanguageFake>::clone(&language),
        digest,
        Arc::new(MemoryFake::default()),
    );
    let placed = PlacedScopes {
        placement: ScopePlacement {
            always,
            ..ScopePlacement::default()
        },
        matcher: MatchingFake.compile(&[]).expect("empty glob set"),
    };
    let records: Vec<_> = files.iter().map(|_| kept).collect();
    let plans = ids
        .iter()
        .map(|_| Plan {
            observation_inputs: Vec::new(),
            observations: Vec::new(),
            parameters: Vec::new(),
            steps: vec![ryo_values::domain::plans::types::PlanStep {
                scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
                operation: ryo_values::domain::plans::types::Operation::Select {
                    from: ryo_values::domain::plan_parts::types::Input::Relation(
                        Relation::Function,
                    ),
                    comparisons: Vec::new(),
                    kept: vec![ColumnIndex(0)],
                },
            }],
            comparison_origins: Vec::new(),
            emit: Emit {
                step: StepId(0),
                subject: ColumnIndex(0),
                key: Vec::new(),
                headings: Vec::new(),
            },
            frontier: Vec::new(),
        })
        .collect::<Vec<_>>();
    let metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    let (program, request) = prepared_test_program(&plans, ids, files);
    let visit = FileVisit {
        store: None,
        setting: RequestVisitSetting {
            source_language: None,
            program,
            services: &request,
            domain: crate::domain::workers::types::VisitSetting {
                unknown: &[],
                metadata: &metadata,
                wanted,
                paths: &[],
                first_bucket: &[],
                ids,
                applicability: &[],
                applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
                placed: &placed,
                records: &records,
                reuse: &ryo_values::domain::stores::types::Reuse::Whole,
                stored: &NOTHING_STORED,
                handed: &NOTHING_HANDED,
                wanted_buckets: &NOTHING_WANTED,
                budget: MemoryBytes(u64::MAX),
                noted: Noting::Unwanted,
                roled: RoleRead::Skips,
                needs,
                answering,
                keeping,
            },
        },
    };
    let mut worker = Worker {
        tables: tables_for(WorkerNumber(0), &[]),
        facts,
        old_rows: None,
        failure: None,
    };
    for _ in 0..repetitions {
        for index in 0..u32::try_from(files.len()).unwrap_or(u32::MAX) {
            service.visit_file(&mut worker, &visit, files, JobNumber(index));
        }
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
        types::{CorpusInventory, PathStat},
    };
    use ryo_values::domain::files::types::{CorpusEntry, FileBytes, FileSize, Nanoseconds};
    struct Refuses {
        stat: bool,
    }
    impl CorpusPort for Refuses {
        fn observe_entry(
            &self,
            path: &FilePath,
        ) -> ryo_language::domain::project_context::types::EntryObservation {
            ryo_language::domain::project_context::types::EntryObservation {
                path: path.clone(),
                state: ryo_language::domain::project_context::types::ObservedEntryState::Unknown,
            }
        }
        fn inventory(&self) -> Result<CorpusInventory, CorpusError> {
            Ok(CorpusInventory(vec![]))
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
                    executable: Default::default(),
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
    inner: Box<dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker>,
    /// Distinguishable same-shape populations selected by read order.
    alternatives: [Vec<ryo_relations::domain::relations::types::FactColumn>; 2],
    /// Source reads remain observable even when their shapes agree.
    reads: std::sync::atomic::AtomicUsize,
}
impl ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker
    for UnstableResidentFacts
{
    fn resident_stability(
        &self,
    ) -> ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability {
        ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability::Unstable
    }
    fn columns_mut(
        &mut self,
    ) -> Result<
        ryo_execution::inbound::use_cases::execution::types::facts::ColumnsMut<'_>,
        ryo_execution::inbound::use_cases::execution::types::facts::FactsError,
    > {
        self.inner.columns_mut()
    }
    fn rows(
        &self,
        relation: Relation,
    ) -> Result<
        ryo_relations::domain::relations::types::ColumnsView<'_>,
        ryo_execution::inbound::use_cases::execution::types::facts::FactsError,
    > {
        use ryo_relations::domain::relations::{service::kinds, types::ColumnsView};
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
        &self,
        bucket: ryo_values::domain::stored_facts::types::Bucket,
        words: ryo_execution::inbound::use_cases::execution::types::facts::WordsToKeep<'_>,
    ) -> Result<(), ryo_execution::inbound::use_cases::execution::types::facts::FactsError> {
        self.inner.keep(bucket, words)
    }
    fn release(
        &mut self,
    ) -> Result<(), ryo_execution::inbound::use_cases::execution::types::facts::FactsError> {
        self.inner.release()
    }
    fn finished(
        self: Box<Self>,
    ) -> Result<
        Box<dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsFinish>,
        ryo_execution::inbound::use_cases::execution::types::facts::FactsError,
    > {
        self.inner.finished()
    }
}

#[test]
fn unstable_same_shape_resident_populations_remain_distinct() {
    use ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker as _;
    use ryo_relations::domain::relations::types::FactColumn;
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
    assert_eq!(
        facts.resident_stability(),
        ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability::Unstable
    );
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
fn file_visits_preserve_stable_unstable_and_unread_provider_outcomes() {
    use crate::domain::render::types::Answering;
    use crate::domain::visit::types::Keeping;
    use ryo_relations::domain::relations::types::FactColumn;
    let (stable, _) = visited_as(
        &["stable.rs"],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Discards,
        Arc::new(DigestFake),
    )
    .expect("stable file visit");
    assert!(stable.failure.is_none());

    let alternate = |value| (0..4).map(|_| FactColumn(vec![value])).collect();
    let facts = UnstableResidentFacts {
        inner: FactsFake::default().worker().expect("facts"),
        alternatives: [alternate(0), alternate(1)],
        reads: std::sync::atomic::AtomicUsize::new(0),
    };
    let (unstable, unstable_language) = visited_as_with_facts(
        &["volatile.rs"],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Discards,
        Arc::new(DigestFake),
        Box::new(facts),
    )
    .expect("unstable file visit");
    assert!(unstable.failure.is_none());
    assert!(unstable_language.parses.load(Ordering::Relaxed) > 0);

    // The corpus fake refuses this path before parsing; no owner is prepared.
    let (unread, _) = visited_as(
        &["unstable.rs"],
        LanguageFake::default(),
        &Answering::Findings,
        Keeping::Discards,
        Arc::new(DigestFake),
    )
    .expect("unread file visit");
    assert!(unread.failure.is_none());
    assert_eq!(unread.tables.unread.len(), 1);
}

#[test]
fn a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket() {
    use super::types::{FileVisit, Worker};

    use crate::domain::visit::types::{Keeping, Noting, RoleRead};
    use crate::outbound::digest::fixtures::fake::DigestCounting;
    use ryo_relations::domain::relations::types::{
        CellKind, FileId, Integer, Name, NamePlace, Offset, Origin, PlaceNumber, TextHash, Value,
        Words,
    };
    use ryo_values::domain::records::types::Kept;
    use ryo_values::domain::stored_facts::types::{ArenaBase, Bucket, KeptWords};
    use ryo_values::domain::table_words::service::{encode_table, pack};
    use ryo_values::domain::table_words::types::WordSlice;
    let plan = Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![ryo_values::domain::plans::types::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
            operation: ryo_values::domain::plans::types::Operation::Select {
                from: ryo_values::domain::plan_parts::types::Input::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(0)],
            },
        }],
        comparison_origins: vec![],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![
            ryo_values::domain::plan_parts::types::Input::Step(StepId(0)),
            ryo_values::domain::plan_parts::types::Input::Step(StepId(0)),
        ],
    };
    let plans = [plan];
    let ids = [RuleId("capture".into())];
    let metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    let (program, request) = prepared_test_program(&plans, &ids, &[]);
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: crate::outbound::matching::fixtures::fake::MatchingFake
            .compile(&[])
            .expect("matcher"),
    };
    let visit = FileVisit {
        store: None,
        setting: RequestVisitSetting {
            source_language: None,
            program,
            services: &request,
            domain: crate::domain::workers::types::VisitSetting {
                metadata: &metadata,
                wanted: &[],
                paths: &[],
                first_bucket: &[Bucket(0)],
                ids: &ids,
                applicability: &[],
                applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
                placed: &placed,
                records: &[],
                reuse: &ryo_values::domain::stores::types::Reuse::Whole,
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
        tables: tables_for(WorkerNumber(0), &metadata),
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
    let proof = |input| ryo_values::domain::records::types::FrontierProof {
        rule: ryo_values::domain::records::types::ProducerRule(0),
        input: ryo_values::domain::records::types::FrontierOrdinal(input),
        rule_digest: ryo_values::domain::stored_facts::types::Digest([0; 16]),
        coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
    };
    let good = Kept {
        bucket: Bucket(0),
        proof: proof(0),
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
                proof: proof(1),
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
        FileOutcome, Handed, HandedWords, Handing, Ran, RecordWords, StagedWords, StagingSession,
    };
    use ryo_values::domain::row_texts::types::FindingRows;
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
                offset: ryo_values::domain::records::types::PayloadOffset(
                    u64::try_from(offset).expect("small count"),
                ),
                length: ryo_values::domain::records::types::PayloadLength(
                    u64::try_from(words.0.len()).expect("size"),
                ),
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
                    proof: crate::outbound::lint_store::fixtures::proof(),
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
        RecordWords::Staged(StagedWords {
            length: ryo_values::domain::records::types::PayloadLength(4096),
            ..
        })
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

    use crate::domain::visit::types::{Keeping, Noting, RoleRead};
    use crate::domain::workers::types::FileOrder;
    use crate::outbound::lint_store::interface::LintStorePort as _;
    use crate::outbound::lint_store::types::{BuildIdentity, CacheDirectory, Store};
    use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value, Words};
    use ryo_values::domain::files::types::{FileNumber, FileSize, Nanoseconds};
    use ryo_values::domain::records::types::{
        FileOutcome, Handed, HandedWords, Handing, Ran, Record, RecordWords, StagedWords,
        StagingSession,
    };
    use ryo_values::domain::row_texts::types::FindingRows;
    use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket, Digest};
    use ryo_values::domain::stores::types::BucketsWanted;
    use ryo_values::domain::table_words::service::{encode_table, pack};
    use ryo_values::domain::table_words::types::WordSlice;

    let directory = tempfile::tempdir().expect("snapshot directory");
    let store = crate::outbound::lint_store::factory::postcard(Store {
        directory: CacheDirectory(directory.path().to_string_lossy().into()),
        build: BuildIdentity("replay-staging".into()),
    });
    let snapshot = store.stored().expect("read snapshot");
    let new_worker = || Worker {
        tables: tables_for(WorkerNumber(0), &[]),
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
                proof: crate::outbound::lint_store::fixtures::proof(),
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
        executable: Default::default(),
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
        proof: crate::outbound::lint_store::fixtures::proof(),
        words: RecordWords::Staged(StagedWords {
            session: StagingSession(u64::MAX),
            offset: ryo_values::domain::records::types::PayloadOffset(u64::MAX),
            length: ryo_values::domain::records::types::PayloadLength(9),
        }),
    });
    let placed = PlacedScopes {
        placement: ScopePlacement::default(),
        matcher: crate::outbound::matching::fixtures::fake::MatchingFake
            .compile(&[])
            .expect("matcher"),
    };
    let wanted = BucketsWanted::These(vec![Bucket(0)]);
    // The proofs the replay checks are those of this program: one rule capturing one frontier input, its first bucket 0.
    let plan = one_capture_plan();
    let plans = [plan];
    let ids = [RuleId("capture".into())];
    let metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    let (program, request) = prepared_test_program(&plans, &ids, &[]);
    let visit = FileVisit {
        store: Some(snapshot.as_ref()),
        setting: RequestVisitSetting {
            source_language: None,
            program,
            services: &request,
            domain: crate::domain::workers::types::VisitSetting {
                metadata: &metadata,
                wanted: &[],
                paths: &[],
                first_bucket: &[Bucket(0)],
                ids: &ids,
                applicability: &[],
                applicability_can_gate: crate::domain::workers::types::Gating::MayGate,
                placed: &placed,
                records: &[],
                reuse: &ryo_values::domain::stores::types::Reuse::Cross(
                    ryo_values::domain::stores::types::MadeAgain::All,
                ),
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
        setting: RequestVisitSetting {
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
    use ryo_values::domain::stored_findings::types::StoredFileNumber;
    use ryo_values::domain::stores::types::{
        KeptRows, OldWords, ReadWordPayload, ReadWords, StoreSnapshot,
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
    persisted.handed.proofs[0].bucket = Bucket(0);
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
        header: ryo_values::domain::records::types::HeaderOffset(u64::MAX),
        offset: ryo_values::domain::records::types::PayloadOffset(u64::MAX),
        length: ryo_values::domain::records::types::PayloadLength(9),
    };
    old_handed.frontier.push(ReadWords {
        file: StoredFileNumber(0),
        bucket: Bucket(1),
        proof: crate::outbound::lint_store::fixtures::proof(),
        words: ReadWordPayload::Old(invalid),
    });
    old_handed.frontier.push(ReadWords {
        file: StoredFileNumber(1),
        bucket: Bucket(0),
        proof: crate::outbound::lint_store::fixtures::proof(),
        words: ReadWordPayload::Old(invalid),
    });
    let mut old_record = record.clone();
    let FileOutcome::Read(ran) = &mut old_record.outcome else {
        panic!("read")
    };
    ran.handed = Handing::Kept;
    let old_visit = FileVisit {
        store: Some(old.as_ref()),
        setting: RequestVisitSetting {
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
        setting: RequestVisitSetting {
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

use ryo_values::domain::rows::types::{RowCount, Table};
#[test]
fn read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes() {
    use crate::domain::render::types::Answering;
    use crate::domain::visit::types::Keeping;
    use crate::outbound::corpus::{
        errors::CorpusError,
        interface::CorpusPort,
        types::{CorpusInventory, PathStat},
    };
    use ryo_values::domain::files::types::{
        CorpusEntry, CorpusInput, FileBytes, FileNumber, FileSize, Nanoseconds,
    };
    use ryo_values::domain::records::types::{FileOutcome, Record, Unreadable};
    use ryo_values::domain::stored_facts::types::{Digest, ReadFailure};
    use std::sync::atomic::AtomicUsize;
    struct ChangingRead {
        reads: AtomicUsize,
        fail_at: usize,
    }
    impl CorpusPort for ChangingRead {
        fn observe_entry(
            &self,
            path: &FilePath,
        ) -> ryo_language::domain::project_context::types::EntryObservation {
            ryo_language::domain::project_context::types::EntryObservation {
                path: path.clone(),
                state: ryo_language::domain::project_context::types::ObservedEntryState::Unknown,
            }
        }
        fn inventory(&self) -> Result<CorpusInventory, CorpusError> {
            unreachable!("explicit test inventory")
        }
        fn stat(&self, _: &FilePath) -> Result<PathStat, CorpusError> {
            unreachable!("no dependency lookup")
        }
        fn read(&self, entry: &CorpusEntry, bytes: &mut FileBytes) -> Result<(), CorpusError> {
            assert!(
                bytes.0.is_empty(),
                "each real attempt starts without earlier source or partial error bytes"
            );
            let at = self.reads.fetch_add(1, Ordering::Relaxed);
            if at == self.fail_at {
                bytes.0.extend_from_slice(b"partial failed bytes");
                Err(CorpusError::Unreadable {
                    path: entry.path.0.to_string(),
                    reason: "read denied".into(),
                })
            } else {
                bytes.0.extend_from_slice(b"fn main() {}");
                Ok(())
            }
        }
    }
    for fail_at in [0, 1, 2] {
        let corpus = Arc::new(ChangingRead {
            reads: AtomicUsize::new(0),
            fail_at,
        });
        let entry = CorpusEntry {
            executable: Default::default(),
            path: FilePath("same.rs".into()),
            size: FileSize(12),
            modified: Nanoseconds(42),
        };
        let kept = Record {
            executable: entry.executable,
            path: entry.path.clone(),
            size: entry.size,
            modified: entry.modified,
            digest: Digest([7; 16]),
            number: FileNumber(0),
            outcome: FileOutcome::Unread {
                reason: Unreadable::Failed(ReadFailure("previous refusal".into())),
                ran: None,
            },
        };
        let (worker, language) = visited_inputs(
            corpus.clone(),
            &[CorpusInput::Available(entry.clone())],
            &[
                Relation::FilesystemReadFailure,
                Relation::FilesystemLineCount,
                Relation::Function,
            ],
            LanguageFake::default(),
            &Answering::Findings,
            Keeping::Keeps,
            Arc::new(DigestFake),
            FactsFake::default().worker().unwrap(),
            Some(&kept),
            3,
        )
        .unwrap();
        assert!(worker.failure.is_none());
        assert_eq!(
            corpus.reads.load(Ordering::Relaxed),
            3,
            "metadata equality cannot certify current readability"
        );
        assert_eq!(
            language.parses.load(Ordering::Relaxed),
            2,
            "a failed disk attempt never reaches the parser"
        );
        if fail_at == 2 {
            assert!(worker.tables.bytes.0.is_empty());
            assert!(
                worker
                    .tables
                    .partials
                    .iter()
                    .any(|partial| partial.relation == Relation::FilesystemLineCount)
            );
            assert!(
                worker
                    .tables
                    .partials
                    .iter()
                    .any(|partial| partial.relation == Relation::Function)
            );
            assert!(
                !worker
                    .tables
                    .partials
                    .iter()
                    .any(|partial| partial.relation == Relation::FilesystemReadFailure)
            );
        } else {
            assert_eq!(worker.tables.bytes.0, b"fn main() {}");
        }
        assert_eq!(worker.tables.unread.len(), 1);
        assert_eq!(
            worker.tables.unrecorded,
            [FilePath("same.rs".into())],
            "the failed visit is retried rather than persisted"
        );
        assert_eq!(
            worker.tables.fresh.len(),
            2,
            "only successfully admitted visits have reusable records"
        );
        if fail_at == 0 {
            use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
            use ryo_values::domain::runs::types::RuleId;
            let excluded = Arc::new(ChangingRead {
                reads: AtomicUsize::new(0),
                fail_at: usize::MAX,
            });
            let needs = [
                Needs::default(),
                Needs {
                    reads: vec![RelationRead {
                        relation: Relation::FilesystemReadFailure,
                        kinds: vec![],
                    }],
                    ..Needs::default()
                },
            ];
            let ids = [RuleId("ordinary".into()), RuleId("read-failure".into())];
            let (replayed, _) = visited_inputs_with_rule_demands(
                excluded.clone(),
                &[CorpusInput::Available(entry.clone())],
                &[Relation::FilesystemReadFailure],
                LanguageFake::default(),
                &Answering::Findings,
                Keeping::Keeps,
                Arc::new(DigestFake),
                FactsFake::default().worker().unwrap(),
                Some(&kept),
                1,
                &ids,
                &needs,
                Mask(vec![1]),
            )
            .unwrap();
            assert!(replayed.failure.is_none());
            assert_eq!(excluded.reads.load(Ordering::Relaxed), 0);

            let included = Arc::new(ChangingRead {
                reads: AtomicUsize::new(0),
                fail_at: usize::MAX,
            });
            let (fresh, _) = visited_inputs_with_rule_demands(
                included.clone(),
                &[CorpusInput::Available(entry.clone())],
                &[Relation::FilesystemReadFailure],
                LanguageFake::default(),
                &Answering::Findings,
                Keeping::Keeps,
                Arc::new(DigestFake),
                FactsFake::default().worker().unwrap(),
                Some(&kept),
                1,
                &ids,
                &needs,
                Mask(vec![2]),
            )
            .unwrap();
            assert!(fresh.failure.is_none());
            assert_eq!(included.reads.load(Ordering::Relaxed), 1);
        }
    }
}

#[test]
fn read_failure_demand_is_limited_to_governing_rules() {
    use crate::domain::reading::service::read_failure_demanded;
    use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
    let wanted = [Relation::FilesystemReadFailure];
    let needs = [
        Needs::default(),
        Needs {
            reads: vec![RelationRead {
                relation: Relation::FilesystemReadFailure,
                kinds: vec![],
            }],
            ..Needs::default()
        },
    ];
    assert_eq!(read_failure_demanded(&wanted, &Mask(vec![1]), &needs, crate::domain::reading::types::RunRuleCount(2)), crate::domain::reading::types::ReadFailureDemand::NotDemanded);
    assert_eq!(read_failure_demanded(&wanted, &Mask(vec![2]), &needs, crate::domain::reading::types::RunRuleCount(2)), crate::domain::reading::types::ReadFailureDemand::Demanded);
    assert_eq!(read_failure_demanded(&wanted, &Mask(vec![1]), &[], crate::domain::reading::types::RunRuleCount(2)), crate::domain::reading::types::ReadFailureDemand::Demanded);
}

#[test]
fn discovery_failure_runs_without_parser_bytes_or_permission_invention() {
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
}

#[test]
fn primary_manifest_read_failure_retains_real_metadata_without_empty_project_source() {
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
}

#[test]
fn project_entry_acquisition_deduplicates_provider_candidates_before_observing() {
    use crate::outbound::corpus::{
        errors::CorpusError,
        interface::CorpusPort,
        types::{CorpusInventory, PathStat},
    };
    use ryo_language::domain::project_context::types::EntryObservation;
    use ryo_values::domain::files::types::{CorpusEntry, FileBytes};
    use std::sync::atomic::AtomicUsize;
    struct CountingCorpus {
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
}

use ryo_execution::inbound::use_cases::metadata::{
    interface::MetadataUseCase, use_case::MetadataUseCaseImpl,
};
