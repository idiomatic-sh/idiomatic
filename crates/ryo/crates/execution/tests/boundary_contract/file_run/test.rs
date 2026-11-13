//! The public file run publishes, refuses and skips through the owner.
use super::super::*;

#[test]
fn public_file_run_publishes_once_and_finishes() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let table = source();
    let reads = Cell::new(0);
    let read = |relation| {
        reads.set(reads.get() + 1);
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
    };
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
                    capture: CaptureIntent::Disabled,
                }],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("open actual file");
    let RootOutcome::Complete(first) = file
        .rule(RuleIndex(0), demand(), &mut symbols)
        .expect("first rule")
        .emit
    else {
        panic!("complete publication required")
    };
    assert_eq!(
        first.cells,
        vec![Value::Node(node_id(FileId(0), Position(2)))]
    );
    assert_eq!(first.count, RowCount(1));
    let after_first = reads.get();
    let RootOutcome::Complete(second) = file
        .rule(RuleIndex(0), demand(), &mut symbols)
        .expect("cached rule")
        .emit
    else {
        panic!("complete cached publication required")
    };
    assert_eq!(first, second);
    assert_eq!(reads.get(), after_first);
    file.finish().expect("finish actual owner");
    request.finish();
}

#[test]
fn public_file_run_refuses_replaced_symbol_population_before_reading() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let reads = Cell::new(0);
    let read = |_| {
        reads.set(reads.get() + 1);
        Err(OperatingError::NotRead)
    };
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
                    capture: CaptureIntent::Disabled,
                }],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("open actual file");
    let mut foreign = Symbols::default();
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut foreign),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(reads.get(), 0);
}

#[test]
fn public_open_refuses_file_outside_request() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let read = |_| Err(OperatingError::NotRead);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    assert!(matches!(
        request.open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: FileId(9),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled
                }],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent
            },
            &mut symbols
        ),
        Err(OperatingError::NotRead)
    ));
}

#[test]
fn public_file_start_skipped_rule_cannot_be_executed() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let table = source();
    let reads = Cell::new(0);
    let read = |relation| {
        reads.set(reads.get() + 1);
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
    };
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
                intents: &[FileRuleIntent::Skipped(
                    ApplicabilityReason::DiagnosticNotApplicable,
                )],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("open actual file");
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(reads.get(), 0);
    file.finish().expect("finish after cancellation");
    request.finish();
}

#[test]
fn public_skip_of_one_shared_rule_preserves_the_other_rules_reader() {
    let engine = engine();
    // Identical valid pure plans share the actual physical producer while IDs,
    // digests and logical applicability remain distinct.
    let program = prepared_rules(&engine, 2);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let table = source();
    let reads = Cell::new(0);
    let read = |relation| {
        reads.set(reads.get() + 1);
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
    };
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
                intents: &[
                    FileRuleIntent::Skipped(ApplicabilityReason::DiagnosticNotApplicable),
                    FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::Disabled,
                    },
                ],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("two actual logical rules");
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(
        reads.get(),
        0,
        "neither skip nor its refused repetition reads the source"
    );
    let RootOutcome::Complete(remaining) = file
        .rule(RuleIndex(1), demand(), &mut symbols)
        .expect("the other logical reader remains valid")
        .emit
    else {
        panic!("shared producer must remain available")
    };
    assert_eq!(remaining.count, RowCount(1));
    assert_eq!(
        remaining.cells,
        vec![Value::Node(node_id(FileId(0), Position(2)))]
    );
    let after_first = reads.get();
    assert!(after_first > 0);
    assert!(
        matches!(
            file.rule(
                RuleIndex(1),
                RuleDemand {
                    emit: EmitDemand::Disabled,
                    observations: ObservationDemand::NotRequired
                },
                &mut symbols
            ),
            Err(OperatingError::NotRun)
        ),
        "an active reader cannot change its bound root demand"
    );
    let RootOutcome::Complete(repeated) = file
        .rule(RuleIndex(1), demand(), &mut symbols)
        .expect("normal repeated publication still works")
        .emit
    else {
        panic!("cached shared publication required")
    };
    assert_eq!(repeated, remaining);
    assert_eq!(reads.get(), after_first);
    file.finish()
        .expect("finish actual owner after shared skip");
    request.finish();
}
