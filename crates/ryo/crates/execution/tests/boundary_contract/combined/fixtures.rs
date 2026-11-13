//! The support of the combined cases: the core's rule intent, the answers as text, and facts whose store refuses.
use crate::boundary_fixture::fixtures::{authored_request, engine};
use crate::file_intents::fixtures::shown;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::FileRules;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use std::fmt::Write as _;

/// A rule whose diagnostic is collected and whose frontiers and observations are captured, as the core runs every rule.
pub(super) const COLLECTED: FileRuleIntent = FileRuleIntent::Run {
    diagnostic: DiagnosticDelivery::Collected,
    observations: ObservationDemand::Required,
    capture: CaptureIntent::DiagnosticAndObservations,
};

/// The rules of a file run as text: each rule's emitted root and how its run went on, then how the file ended.
pub(super) fn answered(rules: FileRules) -> String {
    let (ran, end) = shown(rules);
    let mut text = String::new();
    for each in ran {
        writeln!(
            text,
            "rule {}: {}; {}",
            each.rule.0, each.emitted, each.after
        )
        .unwrap();
    }
    writeln!(text, "file: {end}").unwrap();
    text
}

/// The program of `count` rules, each a select of Function's node (the shared node) followed by `last` of the rule.
pub(super) fn over_a_shared_node(last: &[Option<Operation>]) -> PreparedExecutionProgram {
    let mut input = authored_request(last.len());
    for (plan, step) in crate::boundary_fixture::fixtures::plans(&mut input)
        .iter_mut()
        .zip(last)
    {
        if let Some(operation) = step {
            plan.steps.push(PlanStep {
                scope: StepScope::PerFile,
                operation: operation.clone(),
            });
            plan.emit.step = StepId(1);
        }
    }
    engine()
        .prepare(input)
        .expect("prepare rules over one shared node")
}

/// A projection of the shared node onto `column`.
pub(super) fn project(column: u32) -> Operation {
    Operation::Project {
        from: Input::Step(StepId(0)),
        columns: vec![ColumnIndex(column)],
        distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
    }
}

/// A root as text, as `shown` writes an emitted one.
pub(super) fn root_text(
    root: &ryo_execution::inbound::use_cases::execution::interface::RootOutcome,
) -> String {
    use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
    match root {
        RootOutcome::Complete(table) => format!("complete {table:?}"),
        RootOutcome::Partial { table, affected } => format!("partial {table:?} {affected:?}"),
        RootOutcome::Failed(error) => format!("failed {error:?}"),
        RootOutcome::Unknown(error) => format!("unknown {error:?}"),
        RootOutcome::Deferred { phase } => format!("deferred {phase:?}"),
        RootOutcome::Skipped(_) => "skipped".into(),
        RootOutcome::NoQuery => "no query".into(),
    }
}
