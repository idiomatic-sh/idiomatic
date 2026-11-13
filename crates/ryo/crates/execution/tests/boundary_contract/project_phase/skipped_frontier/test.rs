//! A wholly skipped frontier is unavailable, with no fabricated schema or rows.
use crate::boundary_fixture::fixtures::{authored_request, corpus, engine, run_file};
use crate::project_phase::fixtures::rejected_syntax;
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::CapturedFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::runs::types::{RuleApplicability, SkipReason, Skipped};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::{
    plan_parts::types::{Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
};
use std::sync::atomic::AtomicUsize;

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
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let mut symbols = Symbols::default();
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut facts = TableFacts::new(&[]);
    let rules = run_file(&request, rejected_syntax(&elements), &mut symbols, &mut facts);
    assert!(rules.rules.is_empty(), "the skipped rule does not run");
    assert_eq!(facts.reads(), 0, "rejected syntax has no semantic rows");
    assert!(matches!(rules.end, FileEnd::Finished(_)));
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
