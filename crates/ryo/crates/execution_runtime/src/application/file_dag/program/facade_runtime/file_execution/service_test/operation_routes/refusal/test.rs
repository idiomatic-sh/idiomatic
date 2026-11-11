//! A production route cannot bypass an injected semantic refusal, and fused programs refuse alike.
use super::super::super::super::*;
use super::super::fixtures::chain::{Chain, execute_chain, execute_fixture};
use super::super::fixtures::describe;
use super::fixtures::RefusingSemantics;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::OperationKind;
use std::sync::Arc;
fn production_route_cannot_bypass_an_injected_semantic_refusal() {
    assert!(matches!(
        execute_fixture(OperationKind::Project, false, Arc::new(RefusingSemantics)),
        Err(OperatingError::WrongOperation)
    ));
}
fn all26_fused_and_single_instruction_programs_refuse_with_the_same_error() {
    for &kind in OperationKind::ALL {
        let fused = execute_chain(kind, Chain::Fused, Arc::new(RefusingSemantics));
        let single = execute_chain(kind, Chain::Held, Arc::new(RefusingSemantics));
        assert_eq!(describe(&fused), describe(&single), "{kind:?}");
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ProductionRouteCannotBypassAnInjectedSemanticRefusal,
    All26FusedAndSingleInstructionProgramsRefuseWithTheSameError,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("production_route_cannot_bypass_an_injected_semantic_refusal"),
        word: Word::Succeeds,
        input: || Scenario::ProductionRouteCannotBypassAnInjectedSemanticRefusal,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("all26_fused_and_single_instruction_programs_refuse_with_the_same_error"),
        word: Word::Succeeds,
        input: || Scenario::All26FusedAndSingleInstructionProgramsRefuseWithTheSameError,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ProductionRouteCannotBypassAnInjectedSemanticRefusal => {
            production_route_cannot_bypass_an_injected_semantic_refusal();
            Ok(())
        }
        Scenario::All26FusedAndSingleInstructionProgramsRefuseWithTheSameError => {
            all26_fused_and_single_instruction_programs_refuse_with_the_same_error();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
