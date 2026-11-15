//! The public file run publishes, refuses and skips through the owner.
use super::super::*;

/// The diagnostic of one rule collected.
const COLLECTED: FileRuleIntent = FileRuleIntent::Run {
    diagnostic: DiagnosticDelivery::Collected,
    observations: ObservationDemand::NotRequired,
    capture: CaptureIntent::DiagnosticAndObservations,
};

#[test]
fn public_file_run_publishes_once_and_finishes() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut rules = run_file(
        &request,
        FileRun::of(&[COLLECTED], &elements),
        &mut symbols,
        &mut facts,
    );
    let Ok(RootOutcome::Complete(first)) = root(ran(&mut rules, 0)) else {
        panic!("complete publication required")
    };
    assert_eq!(
        first.cells,
        vec![Value::Node(node_id(FileId(0), Position(2)))]
    );
    assert_eq!(first.count, RowCount(1));
    assert!(facts.reads() > 0);
    assert!(
        matches!(rules.end, FileEnd::Finished(_)),
        "finish actual owner"
    );
    request.finish();
}

#[test]
fn public_open_refuses_file_outside_request() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let mut facts = TableFacts::new(&[]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let rules = run_file(
        &request,
        FileRun {
            file: FileId(9),
            ..FileRun::of(&[COLLECTED], &elements)
        },
        &mut symbols,
        &mut facts,
    );
    assert!(rules.rules.is_empty());
    assert!(matches!(
        rules.end,
        FileEnd::Refused(FileStop::Operating(OperatingError::NotRead))
    ));
    assert_eq!(facts.reads(), 0);
}

#[test]
fn public_file_start_skipped_rule_cannot_be_executed() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let rules = run_file(
        &request,
        FileRun::of(
            &[FileRuleIntent::Skipped(
                ApplicabilityReason::DiagnosticNotApplicable,
            )],
            &elements,
        ),
        &mut symbols,
        &mut facts,
    );
    assert!(rules.rules.is_empty(), "a skipped rule does not run");
    assert_eq!(facts.reads(), 0);
    assert!(
        matches!(rules.end, FileEnd::Finished(_)),
        "finish after cancellation"
    );
    request.finish();
}

#[test]
fn public_skip_of_one_shared_rule_preserves_the_other_rules_reader() {
    let engine = engine();
    // Identical valid pure plans share the actual physical producer while IDs,
    // digests and logical applicability remain distinct.
    let program = prepared_rules(&engine, 2);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut rules = run_file(
        &request,
        FileRun::of(
            &[
                FileRuleIntent::Skipped(ApplicabilityReason::DiagnosticNotApplicable),
                COLLECTED,
            ],
            &elements,
        ),
        &mut symbols,
        &mut facts,
    );
    assert!(
        rules.rules.iter().all(|each| each.rule != RuleIndex(0)),
        "the skipped rule does not run"
    );
    let Ok(RootOutcome::Complete(remaining)) = root(ran(&mut rules, 1)) else {
        panic!("shared producer must remain available")
    };
    assert_eq!(remaining.count, RowCount(1));
    assert_eq!(
        remaining.cells,
        vec![Value::Node(node_id(FileId(0), Position(2)))]
    );
    assert!(facts.reads() > 0);
    assert!(
        matches!(rules.end, FileEnd::Finished(_)),
        "finish actual owner after shared skip"
    );
    request.finish();
}

/// A file's source is asked once, for exactly the relations the request's rules read: a source that serves the functions and
/// the comments is asked for the functions alone when the one rule selects functions.
#[test]
fn a_file_reads_only_the_relations_its_rules_read() {
    use ryo_execution::inbound::use_cases::execution::interface::{
        Arena, CaptureContext, FileRunInput, KindSupport, Partiality, SemanticSupport,
        SourceLanguage,
    };
    use ryo_execution_runtime::outbound::facts::fixtures::sources::SourceFake;
    use ryo_execution_runtime::outbound::facts::types::fact_sources::value::FactSources;
    use ryo_relations::domain::relations::types::{Words, WorkerNumber};
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let mut facts = TableFacts::new(&[]);
    let mut source = SourceFake::serving(&[Relation::Function, Relation::Comment]);
    let mut symbols = Symbols::default();
    let (mut arena, mut places, mut words) = (Vec::new(), Vec::new(), Words::default());
    let first = [ryo_values::domain::stored_facts::types::Bucket(0)];
    let _rules = request.run_file(
        FileRunInput {
            language: SourceLanguage::Recognized,
            file: FileId(0),
            intents: &[COLLECTED],
            source: ryo_values::domain::files::types::SourceView(b""),
            sources: FactSources::read(&mut source, &mut []).expect("one source"),
            active: &[],
            said: &[],
            recovery: ryo_execution::inbound::use_cases::metadata::interface::Recovery::Whole,
            answering: Partiality::Physical,
            semantic: SemanticSupport::Absent,
            kinds: KindSupport::Absent,
            first: &first,
        },
        &mut symbols,
        CaptureContext {
            worker: WorkerNumber(0),
            arena: Arena(&mut arena),
            places: &mut places,
            words: &mut words,
        },
        &mut facts,
    );
    assert_eq!(source.reads, [vec![Relation::Function]]);
    request.finish();
}
