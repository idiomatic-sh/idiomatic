//! The support of the `project_phase` cases and of the groups nested in it.
use crate::boundary_fixture::fixtures::{FileRun, authored_request, ran, run_file};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_execution::inbound::use_cases::execution::interface::RuleAfter;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Relation},
};
use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
};

pub(crate) fn mixed_program(
    engine: &PrepareUseCaseImpl,
    relation: Relation,
) -> PreparedExecutionProgram {
    mixed_program_with(engine, relation, ObservationIdentity::Independent)
}

/// [`mixed_program`] under the observation identity `observations`.
pub(crate) fn mixed_program_with(
    engine: &PrepareUseCaseImpl,
    relation: Relation,
    observations: ObservationIdentity,
) -> PreparedExecutionProgram {
    let named = matches!(observations, ObservationIdentity::Named(_));
    let mut input = authored_request(if named { 2 } else { 1 });
    input.observations = observations;
    crate::boundary_fixture::fixtures::plans(&mut input)[0] = mixed_plan(relation);
    if named {
        use ryo_values::domain::observations::types::{
            BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
            ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
        };
        use ryo_values::domain::plan_parts::types::ObservationBindingId;
        // A named program is a schedule: rule 0 publishes its answer as `files`, which rule 1 binds.
        let columns = vec![ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        }];
        crate::boundary_fixture::fixtures::plans(&mut input)[0]
            .observations
            .push(PlanObservation {
                name: ObservationName("files".into()),
                source: Input::Step(StepId(2)),
                columns: columns.clone(),
                scope: StepScope::CrossFile,
            });
        let consumer = &mut crate::boundary_fixture::fixtures::plans(&mut input)[1];
        consumer.steps = vec![PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::BoundObservation(ObservationBindingId(0)),
                comparisons: vec![],
                kept: vec![],
            },
        }];
        consumer.frontier = vec![Input::BoundObservation(ObservationBindingId(0))];
        consumer.observation_inputs.push(BoundObservationInput {
            name: ObservationBindingName("files".into()),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        });
        let producer = input.rules[0].id.clone();
        input.rules[1].observations.push(ObservationBinding {
            name: ObservationBindingName("files".into()),
            columns,
            producers: vec![ObservationProducer::Output {
                rule: producer,
                output: ObservationName("files".into()),
            }],
        });
    }
    engine
        .prepare(input)
        .expect("compile actual mixed-phase request")
}

/// A plan whose file step selects Function's file, whose project step selects `relation`'s first column, and whose emitted root
/// joins the two after the files; both inputs are its frontier.
pub(crate) fn mixed_plan(relation: Relation) -> ryo_values::domain::plans::types::Plan {
    let mut input = authored_request(1);
    let file_column = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    crate::boundary_fixture::fixtures::plans(&mut input)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(u32::try_from(file_column).unwrap())],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::Relation(relation),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut input)[0].frontier =
        vec![Input::Step(StepId(0)), Input::Relation(relation)];
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .emit
        .step = StepId(2);
    crate::boundary_fixture::fixtures::plans(&mut input).remove(0)
}

/// The kept frontier of rule 0 in file 0, captured as [`captured_in_file_0`] captures it, over `facts`, keyed by its file, every
/// other rule undemanded.
pub(crate) fn captured_in_file_0_of(
    request: &ryo_execution::inbound::use_cases::execution::interface::RequestExecution,
    facts: &TableFacts,
    symbols: &mut Symbols,
) -> Vec<(
    FileId,
    ryo_execution::inbound::use_cases::execution::interface::CapturedFrontier,
)> {
    let elements = ryo_language::domain::elements::types::Elements::default();
    let intents = [
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        },
        FileRuleIntent::Undemanded,
    ];
    captures_of_rule_0(run_file(
        request,
        FileRun::of(&intents, &elements),
        symbols,
        facts,
    ))
    .into_iter()
    .filter_map(|capture| capture.outcome.ok().map(|kept| (FileId(0), kept)))
    .collect()
}

/// The kept frontier of each input of rule 0, captured in file 0 run with its diagnostic collected and its frontiers captured,
/// over facts that hold each relation's table in `rows`, with `publication` read in part.
pub(crate) fn captured_in_file_0(
    request: &ryo_execution::inbound::use_cases::execution::interface::RequestExecution,
    rows: &[(Relation, ryo_values::domain::rows::types::Table)],
    publication: &[ryo_execution::inbound::use_cases::execution::interface::Partial],
    symbols: &mut Symbols,
) -> Vec<ryo_execution::inbound::use_cases::execution::interface::CapturedFrontier> {
    let elements = ryo_language::domain::elements::types::Elements::default();
    let facts = TableFacts::new(
        &rows
            .iter()
            .map(|(relation, table)| (*relation, table))
            .collect::<Vec<_>>(),
    );
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::DiagnosticAndObservations,
    }];
    captures_of_rule_0(run_file(
        request,
        FileRun {
            publication,
            ..FileRun::of(&intents, &elements)
        },
        symbols,
        &facts,
    ))
    .into_iter()
    .map(|capture| capture.outcome.expect("a kept frontier"))
    .collect()
}

/// The frontiers rule 0 of `rules` captured.
fn captures_of_rule_0(
    mut rules: ryo_execution::inbound::use_cases::execution::interface::FileRules,
) -> Vec<ryo_execution::inbound::use_cases::execution::interface::FrontierCapture> {
    match ran(&mut rules, 0).after {
        RuleAfter::Ran { captures } => captures,
        RuleAfter::Unreached => panic!("rule 0's root failed before its frontiers"),
        RuleAfter::Stopped(stop) => panic!("rule 0 stopped before its frontiers: {stop:?}"),
    }
}

/// Recognized file 0 whose syntax was rejected: its one rule is skipped, and it reads no semantic rows.
pub(crate) fn rejected_syntax(
    elements: &ryo_language::domain::elements::types::Elements,
) -> FileRun<'_> {
    FileRun {
        source: b"def broken(:",
        ..FileRun::of(
            &[FileRuleIntent::Skipped(
                ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason::SyntaxRejected,
            )],
            elements,
        )
    }
}
