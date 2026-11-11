//! Missing transport remains an error; explicit syntax rejection and refused acceptance remain partial.
use crate::boundary_fixture::fixtures::{authored_request, corpus, demand, engine, source};
use crate::project_phase::source_facts::fixtures::{CapturedFacts, ProjectFacts};
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::PhaseStop;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Relation, Value},
};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::{
    files::types::{CorpusInput, InputReadFailure},
    runs::types::{RuleApplicability, RuleId, SkipReason, Skipped},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
};
use std::sync::atomic::AtomicUsize;

#[derive(Clone, Copy, Debug)]
enum Case {
    RejectedSyntax,
    Missing,
    /// A required source absent from the frontier, while the edge names only another file inapplicable.
    MissingWhileAnotherFileIsInapplicable,
    /// A file the language left undetermined, the rule refused on it: it has no frame and the edge names it refused.
    UndeterminedSyntax,
    /// The edge names refused a file that carries a frame.
    RefusedFileWithAFrame,
    WrongRule,
    WrongPath,
    AlwaysApplicable,
    ConflictingFrame,
    DuplicateSkip,
    Unreadable,
    AmbiguousPath,
}

#[test]
fn project_frontiers_distinguish_explicit_rejection_from_missing_or_conflicting_inputs() {
    for case in [
        Case::RejectedSyntax,
        Case::Missing,
        Case::MissingWhileAnotherFileIsInapplicable,
        Case::UndeterminedSyntax,
        Case::RefusedFileWithAFrame,
        Case::WrongRule,
        Case::WrongPath,
        Case::AlwaysApplicable,
        Case::ConflictingFrame,
        Case::DuplicateSkip,
        Case::Unreadable,
        Case::AmbiguousPath,
    ] {
        exercise(case);
    }
}

fn exercise(case: Case) {
    let engine = engine();
    let mut authored = authored_request(1);
    authored.rules[0].applicability = if matches!(case, Case::AlwaysApplicable) {
        RuleApplicability::Always
    } else {
        RuleApplicability::AcceptedSyntax
    };
    let rule_id = authored.rules[0].id.clone();
    let column = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].steps[0].operation =
        Operation::Select {
            from: Input::Relation(Relation::Function),
            comparisons: vec![],
            kept: vec![ColumnIndex(
                u32::try_from(column).expect("a column number fits in u32"),
            )],
        };
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .steps
        .push(PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::Step(StepId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        });
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].frontier =
        vec![Input::Step(StepId(0))];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .emit
        .step = StepId(1);
    let prepared = engine.prepare(authored).unwrap();
    let mut corpus = corpus();
    let mut second = corpus[0].clone();
    let CorpusInput::Available(entry) = &mut second else {
        unreachable!()
    };
    entry.path = FilePath(
        if matches!(case, Case::AmbiguousPath) {
            "input.py"
        } else {
            "rejected.py"
        }
        .into(),
    );
    if matches!(case, Case::Unreadable) {
        second = CorpusInput::Unreadable {
            path: FilePath("rejected.py".into()),
            reason: InputReadFailure("read failed".into()),
        };
    }
    corpus.push(second);
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let table = source();
    let read = |_| Ok(SourceRows::Computed(&table));
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::DiagnosticAndObservations,
                }],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .unwrap();
    let mut arena = vec![];
    let mut places = vec![];
    let mut words = Words::default();
    let capture = file
        .capture_frontiers(
            RuleIndex(0),
            Bucket(0),
            &mut symbols,
            CaptureContext {
                worker: WorkerNumber(0),
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words,
            },
        )
        .unwrap()
        .pop()
        .unwrap()
        .outcome
        .unwrap();
    file.finish().unwrap();
    let stored = CapturedFacts {
        captures: vec![(FileId(0), capture)],
        reads: AtomicUsize::new(0),
    };
    if matches!(case, Case::RejectedSyntax) {
        let forbidden_read = |_| -> Result<SourceRows<'_>, OperatingError> {
            panic!("skipped file must not read facts")
        };
        let mut rejected = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: FileId(1),
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected)],
                    source: ryo_values::domain::files::types::SourceView(b"def broken(:"),
                    elements: &elements,
                    read: &forbidden_read,
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )
            .unwrap();
        assert!(matches!(
            rejected.rule(RuleIndex(0), demand(), &mut symbols),
            Err(OperatingError::NotRun)
        ));
        rejected.finish().unwrap();
    }
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "only the cases that frame another file name its path; every other case skips the rejected file"
    )]
    let mut skipped = vec![Skipped {
        rule: if matches!(case, Case::WrongRule) {
            RuleId("different-rule".into())
        } else {
            rule_id
        },
        path: FilePath(
            match case {
                Case::WrongPath => "not-in-corpus.py",
                Case::ConflictingFrame | Case::AmbiguousPath => "input.py",
                _ => "rejected.py",
            }
            .into(),
        ),
        reason: SkipReason::SyntaxRejected,
    }];
    if matches!(
        case,
        Case::Missing
            | Case::MissingWhileAnotherFileIsInapplicable
            | Case::UndeterminedSyntax
            | Case::RefusedFileWithAFrame
    ) {
        skipped.clear();
    }
    if matches!(case, Case::DuplicateSkip) {
        skipped.push(skipped[0].clone());
    }
    let phase = request.project_phase().unwrap();
    let inapplicable = [
        ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles(
            if matches!(case, Case::MissingWhileAnotherFileIsInapplicable) {
                vec![FileId(0)]
            } else {
                Vec::new()
            },
        ),
    ];
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "only the two cases with a refused file name it; every other case refuses none"
    )]
    let refused = [
        ryo_execution::inbound::use_cases::execution::interface::RefusedFiles(match case {
            Case::UndeterminedSyntax => vec![FileId(1)],
            Case::RefusedFileWithAFrame => vec![FileId(0)],
            _ => Vec::new(),
        }),
    ];
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let project_symbols = Symbols::default();
    let partials = Partials::default();
    let occurrences = PhaseRules {
        rules: &[RuleIndex(0)],
        first: &[Bucket(0)],
        corpus: &corpus,
        covered: &[],
        skipped: &skipped,
        inapplicable: &inapplicable,
        refused: &refused,
        facts: &stored,
        project: &project,
        project_symbols: &project_symbols,
        arenas: &[],
    };
    if !matches!(case, Case::RejectedSyntax | Case::UndeterminedSyntax) {
        // The phase scopes the occurrence and prepares its frontiers itself: invalid transport is its frontiers' refusal.
        let batch = phase.evaluate_rules(
            occurrences,
            &mut symbols,
            |_| Ok::<(), OperatingError>(()),
            |_| {
                Ok(PhasePartials {
                    evaluation: partials.clone(),
                    publication: partials.clone(),
                })
            },
            |_| Ok(()),
            |_, outcome, _, _, _| Ok(outcome),
        );
        let ProjectBatch::Complete(done) = batch else {
            panic!("the guard admits every occurrence")
        };
        assert!(
            matches!(done.as_slice(), [Err(PhaseStop::Frontiers(_))]),
            "invalid transport accepted: {case:?}"
        );
        return;
    }
    let RootOutcome::Partial { table, affected } = crate::boundary_fixture::fixtures::project_root(
        &phase,
        occurrences,
        &partials,
        &partials,
        &mut symbols,
    )
    .expect("explicit rejection or refusal accounts for absent transport")
    .emit
    else {
        panic!("a skipped or refused partition must not become complete empty input: {case:?}")
    };
    assert_eq!(table.kinds, [CellKind::File]);
    // The accepted file's row is answered; only the skipped or refused file is uncertain.
    assert_eq!(
        table.cells,
        [Value::File(FileId(0))],
        "{case:?}: the accepted file's row"
    );
    assert_eq!(table.count, RowCount(1));
    assert_eq!(
        affected,
        [Some(FileId(1))],
        "{case:?}: only the skipped or refused file is uncertain"
    );
}

#[test]
fn wholly_skipped_frontier_is_unavailable_without_fabricating_a_schema_or_rows() {
    let engine = engine();
    let mut authored = authored_request(1);
    authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    let rule_id = authored.rules[0].id.clone();
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .steps
        .push(PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::Step(StepId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        });
    crate::boundary_fixture::fixtures::plans(&mut authored)[0]
        .emit
        .step = StepId(1);
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].frontier =
        vec![Input::Step(StepId(0))];
    let prepared = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let mut symbols = Symbols::default();
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| -> Result<SourceRows<'_>, OperatingError> {
        panic!("rejected syntax has no semantic rows")
    };
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected)],
                source: ryo_values::domain::files::types::SourceView(b"def broken(:"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .unwrap();
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols),
        Err(OperatingError::NotRun)
    ));
    file.finish().unwrap();
    let skipped = [Skipped {
        rule: rule_id,
        path: FilePath("input.py".into()),
        reason: SkipReason::SyntaxRejected,
    }];
    let phase = request.project_phase().unwrap();
    let stored = CapturedFacts {
        captures: vec![],
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let project_symbols = Symbols::default();
    let partials = Partials::default();
    let outcome = crate::boundary_fixture::fixtures::project_root(
        &phase,
        PhaseRules {
            rules: &[RuleIndex(0)],
            first: &[Bucket(0)],
            corpus: &corpus,
            covered: &[],
            skipped: &skipped,
            inapplicable: &[
                ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default(
                ),
            ],
            refused: &[
                ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default(),
            ],
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
            arenas: &[],
        },
        &partials,
        &partials,
        &mut symbols,
    );
    assert!(matches!(outcome.unwrap().emit,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { reason })
        if reason == "every source partition was explicitly skipped"));
}
