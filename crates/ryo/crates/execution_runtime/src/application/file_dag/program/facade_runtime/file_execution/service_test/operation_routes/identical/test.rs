//! Fused and single-instruction programs publish identical rows in identical order.
use super::super::super::super::*;
use super::super::fixtures::chain::{Chain, execute_chain};
use super::super::fixtures::describe;
use super::fixtures::engine;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::executable_program::fusion::types::{Barrier, Fusibility};
use ryo_values::domain::query_operators::types::OperationKind;
fn all26_fused_and_single_instruction_programs_publish_identical_rows_in_identical_order() {
    for &kind in OperationKind::ALL {
        let fused = execute_chain(kind, Chain::Fused, engine());
        let single = execute_chain(kind, Chain::Held, engine());
        assert!(
            !fused.continuations.is_empty(),
            "{kind:?} continues something"
        );
        assert!(
            fused
                .continuations
                .iter()
                .all(|class| *class == Some(Fusibility::Fusable)),
            "{kind:?}: the pure projection fuses into its sole-reader producer"
        );
        assert!(
            single
                .continuations
                .iter()
                .all(|class| *class == Some(Fusibility::Barrier(Barrier::SharedConsumer))),
            "{kind:?}: a second reader keeps the producer's publication"
        );
        assert!(fused.outcome.is_ok(), "{kind:?}: {}", describe(&fused));
        assert_eq!(describe(&fused), describe(&single), "{kind:?}");
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    All26FusedAndSingleInstructionProgramsPublishIdenticalRowsInIdenticalOrder,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(
        "all26_fused_and_single_instruction_programs_publish_identical_rows_in_identical_order",
    ),
    word: Word::Succeeds,
    input: || Scenario::All26FusedAndSingleInstructionProgramsPublishIdenticalRowsInIdenticalOrder,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::All26FusedAndSingleInstructionProgramsPublishIdenticalRowsInIdenticalOrder => {
            all26_fused_and_single_instruction_programs_publish_identical_rows_in_identical_order();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
