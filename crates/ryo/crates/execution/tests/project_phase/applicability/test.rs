//! Missing transport remains an error; explicit syntax rejection remains partial.
use super::super::*;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::{
    files::types::{CorpusInput, InputReadFailure},
    runs::types::{RuleApplicability, RuleId, SkipReason, Skipped},
};

#[derive(Clone, Copy, Debug)]
enum Case {
    RejectedSyntax,
    Missing,
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
    authored.plans[0].steps[0].operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(column as u32)],
    };
    authored.plans[0].steps.push(PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::Step(StepId(0)),
            comparisons: vec![],
            kept: vec![],
        },
    });
    authored.plans[0].frontier = vec![Input::Step(StepId(0))];
    authored.plans[0].emit.step = StepId(1);
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
    let request = engine.begin(&prepared, &corpus);
    let table = source();
    let read = |_| Ok(SourceRows::Computed(&table));
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::DiagnosticAndObservations,
                }],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
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
                arena: &mut arena,
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
                    resident_stability: facts::ResidentStability::Stable,
                    file: FileId(1),
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected)],
                    source: b"def broken(:",
                    elements: &elements,
                    read: &forbidden_read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .unwrap();
        assert!(matches!(
            rejected.rule(RuleIndex(0), demand(), &mut symbols),
            Err(OperatingError::NotRun)
        ));
        assert_eq!(rejected.walk_counts().passes.0, 0);
        rejected.finish().unwrap();
    }
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
    if matches!(case, Case::Missing) {
        skipped.clear();
    }
    if matches!(case, Case::DuplicateSkip) {
        skipped.push(skipped[0].clone());
    }
    let phase = request.project_phase().unwrap();
    let scope = phase
        .project_scope(RuleIndex(0), &corpus, &[], &skipped)
        .unwrap();
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let project_symbols = Symbols::default();
    let ordinary = request.prepare_frontiers(
        FrontierRequest {
            demand: FrontierDemand::Diagnostic,
            scope: &scope,
            rule: RuleIndex(0),
            first: Bucket(0),
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
        },
        &mut symbols,
    );
    if !matches!(case, Case::RejectedSyntax) {
        assert!(ordinary.is_err(), "invalid transport accepted: {case:?}");
        return;
    }
    let ordinary = ordinary.expect("explicit rejection accounts for absent transport");
    let partials = Partials::default();
    let mut run = request
        .open_project(
            ProjectInput {
                scope: &scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: &[],
            },
            &mut symbols,
        )
        .unwrap();
    let RootOutcome::Partial { table, affected } =
        run.rule(RuleIndex(0), demand(), &mut symbols).unwrap().emit
    else {
        panic!("skipped partition must not become complete empty input")
    };
    assert_eq!(table.kinds, [CellKind::File]);
    assert_eq!(table.cells, [Value::File(FileId(0))]);
    assert_eq!(table.count, RowCount(1));
    assert_eq!(affected, [Some(FileId(1))]);
    run.finish();
}

#[test]
fn wholly_skipped_frontier_is_unavailable_without_fabricating_a_schema_or_rows() {
    let engine = engine();
    let mut authored = authored_request(1);
    authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    let rule_id = authored.rules[0].id.clone();
    authored.plans[0].steps.push(PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::Step(StepId(0)),
            comparisons: vec![],
            kept: vec![],
        },
    });
    authored.plans[0].emit.step = StepId(1);
    authored.plans[0].frontier = vec![Input::Step(StepId(0))];
    let prepared = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let mut symbols = Symbols::default();
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| -> Result<SourceRows<'_>, OperatingError> {
        panic!("rejected syntax has no semantic rows")
    };
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected)],
                source: b"def broken(:",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .unwrap();
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(file.walk_counts().passes.0, 0);
    file.finish().unwrap();
    let skipped = [Skipped {
        rule: rule_id,
        path: FilePath("input.py".into()),
        reason: SkipReason::SyntaxRejected,
    }];
    let phase = request.project_phase().unwrap();
    let scope = phase
        .project_scope(RuleIndex(0), &corpus, &[], &skipped)
        .unwrap();
    let stored = CapturedFacts {
        captures: vec![],
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let project_symbols = Symbols::default();
    let ordinary = request
        .prepare_frontiers(
            FrontierRequest {
                demand: FrontierDemand::Diagnostic,
                scope: &scope,
                rule: RuleIndex(0),
                first: Bucket(0),
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
            },
            &mut symbols,
        )
        .unwrap();
    let partials = Partials::default();
    let mut run = request
        .open_project(
            ProjectInput {
                scope: &scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: &[],
            },
            &mut symbols,
        )
        .unwrap();
    assert!(
        matches!(run.rule(RuleIndex(0), demand(), &mut symbols).unwrap().emit,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { reason })
        if reason == "every source partition was explicitly skipped")
    );
    run.finish();
}
