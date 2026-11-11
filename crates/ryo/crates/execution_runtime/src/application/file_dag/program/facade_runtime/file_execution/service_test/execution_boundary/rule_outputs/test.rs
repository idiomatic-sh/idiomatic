//! Rule outputs outlive workspace drops, and evaluation errors leave no mutable state behind.
use super::super::super::super::*;
use super::fixtures::output_requests;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value as Cell};
fn large_tiny_and_empty_rule_outputs_are_owned_beyond_workspace_drop() {
    let outputs = [("large", 4096), ("tiny", 1), ("empty", 0), ("small", 3)];
    let counts = outputs.map(|(_, count)| count);
    let (prepared, services) = output_requests(&outputs, false);
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }; 4],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let mut outputs = Vec::new();
    for (index, &count) in counts.iter().enumerate() {
        let outcome = file
            .rule(
                RuleIndex(index),
                RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::NotRequired,
                },
                &read,
                &mut symbols,
            )
            .unwrap();
        let RootOutcome::Complete(table) = outcome.emit else {
            panic!("owned output required")
        };
        assert_eq!(
            table.kinds,
            [CellKind::Number],
            "empty outputs retain the actual prepared schema"
        );
        assert_eq!(table.count.0, count);
        assert_eq!(table.cells, vec![Cell::Number(Integer(7)); count]);
        outputs.push(table);
    }
    let pointers = outputs
        .iter()
        .map(|table| table.cells.as_ptr())
        .collect::<Vec<_>>();
    file.finish().unwrap();
    for ((table, &count), pointer) in outputs.iter().zip(&counts).zip(pointers) {
        assert_eq!(table.count.0, count);
        assert_eq!(table.kinds, [CellKind::Number]);
        assert_eq!(table.cells, vec![Cell::Number(Integer(7)); count]);
        assert_eq!(
            table.cells.as_ptr(),
            pointer,
            "the returned table keeps its own allocation after actual owner finish"
        );
    }
}
fn evaluation_errors_do_not_leave_mutable_state_in_the_next_rule() {
    let (prepared, services) = output_requests(&[("failing", 2), ("following", 2)], true);
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }; 2],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let demand = || RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let failed = file
        .rule(RuleIndex(0), demand(), &read, &mut symbols)
        .unwrap();
    assert!(matches!(
        failed.emit,
        RootOutcome::Failed(OperatingError::Table(
            ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: 99 }
        ))
    ));
    let next = file
        .rule(RuleIndex(1), demand(), &read, &mut symbols)
        .unwrap();
    let RootOutcome::Complete(table) = next.emit else {
        panic!("independent next rule must complete")
    };
    assert_eq!(table.count.0, 2);
    assert_eq!(table.kinds, [CellKind::Number]);
    assert_eq!(table.cells, [Cell::Number(Integer(7)); 2]);
    let repeated = file
        .rule(RuleIndex(0), demand(), &read, &mut symbols)
        .unwrap();
    assert!(
        matches!(
            repeated.emit,
            RootOutcome::Failed(OperatingError::Table(
                ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: 99 }
            ))
        ),
        "the earlier failure remains exact after a later successful invocation"
    );
    file.finish().unwrap();
    assert_eq!(table.cells, [Cell::Number(Integer(7)); 2]);
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    LargeTinyAndEmptyRuleOutputsAreOwnedBeyondWorkspaceDrop,
    EvaluationErrorsDoNotLeaveMutableStateInTheNextRule,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("large_tiny_and_empty_rule_outputs_are_owned_beyond_workspace_drop"),
        word: Word::Succeeds,
        input: || Scenario::LargeTinyAndEmptyRuleOutputsAreOwnedBeyondWorkspaceDrop,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("evaluation_errors_do_not_leave_mutable_state_in_the_next_rule"),
        word: Word::Succeeds,
        input: || Scenario::EvaluationErrorsDoNotLeaveMutableStateInTheNextRule,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::LargeTinyAndEmptyRuleOutputsAreOwnedBeyondWorkspaceDrop => {
            large_tiny_and_empty_rule_outputs_are_owned_beyond_workspace_drop();
            Ok(())
        }
        Scenario::EvaluationErrorsDoNotLeaveMutableStateInTheNextRule => {
            evaluation_errors_do_not_leave_mutable_state_in_the_next_rule();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
