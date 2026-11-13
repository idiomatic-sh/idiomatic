//! Each case runs file 0 through `run_file` and compares every rule's answer and how the file ended with what main answered.
use super::fixtures::{COLLECTED, answered, over_a_shared_node, project, root_text};
use crate::boundary_fixture::fixtures::{
    FileRun, authored_request, corpus, engine, run_file, source,
};
use crate::file_intents::fixtures::{named_request, partial};
use crate::shared_walk::fixtures::{Selector, compared, selection, text, tree};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RuleAfter;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_merged::Buckets;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::CapturedFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::fixtures::kinds;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId, StepScope, Test};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::rows::types::{RowCount, Table};
use ryo_values::domain::stored_facts::types::Bucket;
use std::fmt::Write as _;
use std::sync::atomic::AtomicUsize;

#[test]
fn shared_node_with_a_skipped_and_a_failed_rule() {
    let program = over_a_shared_node(&[None, None, Some(project(99))]);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let intents = [
        FileRuleIntent::Skipped(ApplicabilityReason::DiagnosticNotApplicable),
        COLLECTED,
        COLLECTED,
    ];
    let rules = run_file(
        &request,
        FileRun::of(&intents, &elements),
        &mut Symbols::default(),
        &facts,
    );
    request.finish();
    assert_eq!(
        answered(rules),
        "rule 1: complete Table { kinds: [Node], cells: [Node(NodeId(2))], count: RowCount(1) }; ran, 0 captures\nrule 2: failed Table(NoSuchColumn { index: 99 }); unreached\nfile: unfinished\n",
        "the answers main gave"
    );
}

#[test]
fn partial_read_with_an_antijoin_and_an_absence() {
    let mut input = authored_request(2);
    let plans = crate::boundary_fixture::fixtures::plans(&mut input);
    plans[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Antijoin {
                left: Input::Step(StepId(0)),
                right: Input::Relation(Relation::Comment),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
        },
    ];
    plans[0].emit.step = StepId(1);
    plans[1].steps[0].operation = Operation::Select {
        from: Input::Relation(Relation::Comment),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    };
    let program = engine()
        .prepare(input)
        .expect("prepare an antijoin and an absence");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let functions = source();
    let comments = Table {
        kinds: kinds(Relation::Comment).to_vec(),
        count: RowCount(0),
        cells: vec![],
    };
    let facts = TableFacts::new(&[
        (Relation::Function, &functions),
        (Relation::Comment, &comments),
    ]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read_in_part = [partial(Relation::Comment)];
    let rules = run_file(
        &request,
        FileRun {
            evaluation: &read_in_part,
            publication: &read_in_part,
            ..FileRun::of(&[COLLECTED, COLLECTED], &elements)
        },
        &mut Symbols::default(),
        &facts,
    );
    request.finish();
    assert_eq!(
        answered(rules),
        "rule 0: partial Table { kinds: [Node], cells: [], count: RowCount(0) } [Some(FileId(0))]; ran, 0 captures\nrule 1: partial Table { kinds: [Node], cells: [], count: RowCount(0) } [Some(FileId(0))]; ran, 0 captures\nfile: finished\n",
        "the answers main gave"
    );
}

#[test]
fn observation_only_demand_beside_a_running_rule() {
    let program = engine()
        .prepare(named_request())
        .expect("a named producer and its consumer");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let intents = [
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Disabled,
            observations: ObservationDemand::Required,
            capture: CaptureIntent::Observations,
        },
        COLLECTED,
    ];
    let rules = run_file(
        &request,
        FileRun::of(&intents, &elements),
        &mut Symbols::default(),
        &facts,
    );
    request.finish();
    assert_eq!(
        answered(rules),
        "rule 0: withheld; ran, 1 captures\nrule 1: deferred CrossFile; ran, 1 captures\nfile: finished\n",
        "the answers main gave"
    );
}

#[test]
fn capture_storage_failure_after_a_shared_node() {
    let mut input = authored_request(2);
    for plan in crate::boundary_fixture::fixtures::plans(&mut input) {
        plan.frontier = vec![Input::Step(StepId(0))];
    }
    let program = engine()
        .prepare(input)
        .expect("prepare two rules capturing one shared node");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    // The facts serve the shared node's rows and refuse to keep any captured frontier.
    let table = source();
    let facts = TableFacts::new(&[(Relation::Function, &table)]).refusing_keep();
    let elements = ryo_language::domain::elements::types::Elements::default();
    let rules = run_file(
        &request,
        FileRun::of(&[COLLECTED, COLLECTED], &elements),
        &mut Symbols::default(),
        &facts,
    );
    request.finish();
    assert_eq!(
        answered(rules),
        "rule 0: complete Table { kinds: [Node], cells: [Node(NodeId(2))], count: RowCount(1) }; stopped Facts(Store { reason: \"the store refused the words\" })\nfile: unfinished\n",
        "the answers main gave"
    );
}

#[test]
fn failing_pipeline_stage_with_an_earlier_failing_stage_on_a_later_row() {
    let (engine, _) = crate::shared_walk::fixtures::engine();
    let mut input = authored_request(1);
    // The first stage fails only on the file's last row, whose text is not a pattern; the second stage fails on its first row.
    crate::boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                comparisons: selection(Selector::DynamicRegex),
                kept: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Step(StepId(0)),
                comparisons: vec![compared(Test::Matches, 2, text("["))],
                kept: vec![],
            },
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .emit
        .step = StepId(1);
    let program = engine.prepare(input).expect("prepare a fused pipeline");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let (elements, mut symbols) = tree(FileId(0), b"ab(");
    let rules = run_file(
        &request,
        FileRun {
            source: b"ab(",
            ..FileRun::of(&[COLLECTED], &elements)
        },
        &mut symbols,
        &TableFacts::new(&[]),
    );
    request.finish();
    assert_eq!(
        answered(rules),
        "rule 0: failed InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" }; unreached\nfile: unfinished\n",
        "the answers main gave"
    );
}

#[test]
fn project_root_shared_by_two_consumers_with_one_failing() {
    let engine = engine();
    let relation = Relation::ModuleIdentity;
    let mut input = authored_request(2);
    let plans = crate::boundary_fixture::fixtures::plans(&mut input);
    let shared = crate::project_phase::fixtures::mixed_plan(relation);
    plans[0] = shared.clone();
    plans[1] = shared;
    // The second consumer's project join answers a column its input does not have.
    if let Some(PlanStep {
        operation: Operation::Join { output, .. },
        ..
    }) = plans[1].steps.last_mut()
    {
        output[0].column = ColumnIndex(99);
    }
    let program = engine
        .prepare(input)
        .expect("prepare two consumers of one project root");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut file = run_file(
        &request,
        FileRun {
            first: &[Bucket(0), Bucket(1)],
            ..FileRun::of(&[COLLECTED, COLLECTED], &elements)
        },
        &mut symbols,
        &facts,
    );
    let mut text = String::new();
    let mut stored = Vec::new();
    for rule in [0, 1] {
        let ran = crate::boundary_fixture::fixtures::ran(&mut file, rule);
        let RuleAfter::Ran { captures } = ran.after else {
            panic!("rule {rule} captures its file frontier")
        };
        for capture in captures {
            stored.push(CapturedFacts {
                captures: vec![(FileId(0), capture.outcome.expect("a kept frontier"))],
                reads: AtomicUsize::new(0),
            });
        }
    }
    let stored = Buckets(stored);
    let project = ProjectFacts::new(relation);
    let project_symbols = Symbols::default();
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0), RuleIndex(1)],
        partial: vec![],
    }];
    let phase = request.project_phase().expect("the project phase");
    let outcomes = crate::boundary_fixture::fixtures::project_roots(
        &phase,
        PhaseRules {
            rules: &[RuleIndex(0), RuleIndex(1)],
            first: &[Bucket(0), Bucket(1)],
            corpus: &corpus,
            covered: &covered,
            skipped: &[],
            inapplicable: &[Default::default(), Default::default()],
            refused: &[Default::default(), Default::default()],
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
            arenas: &[],
        },
        &Partials::default(),
        &Partials::default(),
        &mut symbols,
    );
    for (rule, outcome) in outcomes.into_iter().enumerate() {
        let shown = match outcome.map(|outcome| outcome.emit) {
            Ok(root) => root_text(&root),
            Err(error) => format!("refused {error:?}"),
        };
        writeln!(text, "project rule {rule}: {shown}").unwrap();
    }
    phase.finish().expect("release the project phase");
    request.finish();
    assert_eq!(
        text,
        "project rule 0: complete Table { kinds: [File], cells: [File(FileId(0))], count: RowCount(1) }\nproject rule 1: failed Table(NoSuchColumn { index: 99 })\n",
        "the answers main gave"
    );
}
