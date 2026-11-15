//! File-start declarations are enforced through every public execution route.
use super::super::*;
use ryo_values::domain::runs::types::ApplicabilityRefusal;

#[test]
fn every_prepared_rule_requires_exactly_one_file_start_intent() {
    let engine = engine();
    let program = prepared_rules(&engine, 2);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut facts = TableFacts::new(&[]);
    let mut symbols = Symbols::default();
    for intents in [
        &[][..],
        &[FileRuleIntent::Undemanded][..],
        &[FileRuleIntent::Undemanded; 3][..],
    ] {
        let rules = run_file(
            &request,
            FileRun::of(intents, &elements),
            &mut symbols,
            &mut facts,
        );
        assert!(rules.rules.is_empty());
        assert!(matches!(
            rules.end,
            FileEnd::Refused(FileStop::Operating(OperatingError::ShapeMismatch))
        ));
    }
    assert_eq!(
        facts.reads(),
        0,
        "malformed intent vectors cannot read a provider"
    );
    request.finish();
}

#[test]
fn absent_skipped_and_refused_rules_cannot_acquire_any_root() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut facts = TableFacts::new(&[]);
    for intent in [
        FileRuleIntent::Undemanded,
        FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected),
        FileRuleIntent::Refused(ApplicabilityRefusal::SyntaxUndetermined),
        FileRuleIntent::Refused(ApplicabilityRefusal::MetadataUnavailable),
    ] {
        let intents = [intent];
        let mut symbols = Symbols::default();
        let rules = run_file(
            &request,
            FileRun::of(&intents, &elements),
            &mut symbols,
            &mut facts,
        );
        assert!(rules.rules.is_empty(), "an undeclared root does not run");
        assert!(matches!(rules.end, FileEnd::Finished(_)));
    }
    assert_eq!(facts.reads(), 0, "undeclared roots cannot read a provider");
    request.finish();
}

#[test]
fn rule_demand_is_immutable_and_capture_cannot_be_added_after_open() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let elements = ryo_language::domain::elements::types::Elements::default();
    let table = source();
    let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }];
    let mut symbols = Symbols::default();
    let mut rules = run_file(
        &request,
        FileRun::of(&intents, &elements),
        &mut symbols,
        &mut facts,
    );
    let ran = ran(&mut rules, 0);
    assert!(matches!(
        ran.emit,
        EmittedRoot::Delivered(RootOutcome::Complete(_))
    ));
    assert!(
        matches!(
            ran.after,
            RuleAfter::Stopped(FileStop::Operating(OperatingError::NotRun))
        ),
        "a rule whose capture is disabled refuses the capture after its root"
    );
    assert!(facts.reads() > 0);
    assert!(matches!(rules.end, FileEnd::Unfinished));
    request.finish();
}
