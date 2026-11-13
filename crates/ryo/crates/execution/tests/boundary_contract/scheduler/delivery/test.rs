//! A file's second run reads its own facts and never a cached row, and an undemanded rule is never scheduled.
use crate::boundary_fixture::fixtures::{
    FileRun, authored_request, corpus, engine, ran, root, run_file, source,
};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::FileStop;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::*;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
};

#[test]
fn a_second_run_of_a_file_reads_its_own_facts_and_never_a_cached_row() {
    let engine = engine();
    let mut authored = authored_request(2);
    for plan in crate::boundary_fixture::fixtures::plans(&mut authored) {
        plan.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(0)),
            },
        });
        plan.emit.step = StepId(1);
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let elements = Elements::default();
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }; 2];
    let mut symbols = Symbols::default();
    // The first run reads the provider once for both rules' shared dependency; the second run of the same file reads the facts
    // of its own run, which refuse, and no rule delivers a row of the first.
    let mut first = TableFacts::new(&[(Relation::Function, &table)]);
    let mut rules = run_file(
        &request,
        FileRun::of(&intents, &elements),
        &mut symbols,
        &mut first,
    );
    for index in [0, 1] {
        let Ok(RootOutcome::Complete(rows)) = root(ran(&mut rules, index)) else {
            panic!("rule {index} delivers its terminal publication")
        };
        assert_eq!(rows.cells, [table.cells[0]]);
    }
    assert!(first.reads() > 0);
    let mut second = TableFacts::new(&[]);
    let mut rules = run_file(
        &request,
        FileRun::of(&intents, &elements),
        &mut symbols,
        &mut second,
    );
    assert!(
        matches!(root(ran(&mut rules, 0)), Err(FileStop::Facts(_))),
        "a retired run must not supply a cached row"
    );
    assert!(
        second.reads() > 0,
        "the fresh run admitted the current provider"
    );
    assert!(matches!(rules.end, FileEnd::Unfinished));
    request.finish();
}

#[test]
fn an_unbound_rule_cannot_schedule_an_independent_provider_after_a_failed_sibling() {
    let engine = engine();
    let mut authored = authored_request(2);
    crate::boundary_fixture::fixtures::plans(&mut authored)[1].steps[0].operation =
        Operation::Select {
            from: Input::Relation(Relation::Identifier),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        };
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    // No relation is held: rule 0's read of Function is refused. The undemanded sibling's Identifier is never read.
    let mut facts = TableFacts::new(&[]);
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let intents = [
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        },
        FileRuleIntent::Undemanded,
    ];
    let mut rules = run_file(
        &request,
        FileRun::of(&intents, &elements),
        &mut symbols,
        &mut facts,
    );
    assert!(matches!(root(ran(&mut rules, 0)), Err(FileStop::Facts(_))));
    assert!(
        rules.rules.iter().all(|each| each.rule != RuleIndex(1)),
        "the undemanded sibling is not scheduled"
    );
    assert!(
        facts
            .history()
            .iter()
            .all(|relation| *relation == Relation::Function),
        "undeclared sibling cannot be prepared"
    );
    request.finish();
}
