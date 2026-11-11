//! Every operation kind and route of the compiled corpus has a fusion classification.
use super::super::super::super::fixtures::operations::corpus::compiled_operations;
use super::fixtures::{Refusal, verify};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::executable_program::fusion::types::{Barrier, Fusibility};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::query_operators::types::OperationKind;

fn every_operation_kind_and_route_has_a_fusion_classification() {
    let corpus = compiled_operations();
    verify(&corpus, &|logical| corpus.program.fusibility(logical)).unwrap();
}

fn an_unclassified_instruction_is_refused() {
    let corpus = compiled_operations();
    let victim = corpus
        .program
        .regions()
        .iter()
        .flat_map(|region| region.instructions().iter())
        .find(|instruction| kind_of(&instruction.operation()) == OperationKind::Join)
        .unwrap()
        .logical();
    let lookup = |logical| {
        (logical != victim)
            .then(|| corpus.program.fusibility(logical))
            .flatten()
    };
    assert_eq!(verify(&corpus, &lookup), Err(Refusal::Unclassified));
}

fn a_kind_relabelled_as_fusable_is_refused() {
    let corpus = compiled_operations();
    let rows = corpus
        .program
        .regions()
        .iter()
        .flat_map(|region| region.instructions().iter())
        .find(|instruction| kind_of(&instruction.operation()) == OperationKind::Rows)
        .unwrap()
        .logical();
    let lookup = |logical| {
        if logical == rows {
            Some(Fusibility::Fusable)
        } else {
            corpus.program.fusibility(logical)
        }
    };
    assert_eq!(verify(&corpus, &lookup), Err(Refusal::Mismatch));
}

fn a_fused_continuation_relabelled_as_a_barrier_is_refused() {
    let corpus = compiled_operations();
    let fused = corpus
        .program
        .regions()
        .iter()
        .find(|region| region.instructions().len() > 1)
        .expect("the corpus fuses one continuation")
        .instructions()[1]
        .logical();
    let lookup = |logical| {
        if logical == fused {
            Some(Fusibility::Barrier(Barrier::SharedConsumer))
        } else {
            corpus.program.fusibility(logical)
        }
    };
    assert_eq!(verify(&corpus, &lookup), Err(Refusal::Mismatch));
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    EveryOperationKindAndRouteHasAFusionClassification,
    AnUnclassifiedInstructionIsRefused,
    AKindRelabelledAsFusableIsRefused,
    AFusedContinuationRelabelledAsABarrierIsRefused,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_operation_kind_and_route_has_a_fusion_classification"),
        word: Word::Succeeds,
        input: || Scenario::EveryOperationKindAndRouteHasAFusionClassification,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_unclassified_instruction_is_refused"),
        word: Word::Succeeds,
        input: || Scenario::AnUnclassifiedInstructionIsRefused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_kind_relabelled_as_fusable_is_refused"),
        word: Word::Succeeds,
        input: || Scenario::AKindRelabelledAsFusableIsRefused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_fused_continuation_relabelled_as_a_barrier_is_refused"),
        word: Word::Succeeds,
        input: || Scenario::AFusedContinuationRelabelledAsABarrierIsRefused,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::EveryOperationKindAndRouteHasAFusionClassification => {
            every_operation_kind_and_route_has_a_fusion_classification();
            Ok(())
        }
        Scenario::AnUnclassifiedInstructionIsRefused => {
            an_unclassified_instruction_is_refused();
            Ok(())
        }
        Scenario::AKindRelabelledAsFusableIsRefused => {
            a_kind_relabelled_as_fusable_is_refused();
            Ok(())
        }
        Scenario::AFusedContinuationRelabelledAsABarrierIsRefused => {
            a_fused_continuation_relabelled_as_a_barrier_is_refused();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
