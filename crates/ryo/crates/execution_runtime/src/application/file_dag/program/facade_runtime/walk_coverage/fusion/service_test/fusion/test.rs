//! Every operation kind and route of the compiled corpus has a fusion classification.
use super::super::super::super::fixtures::operations::corpus::compiled_operations;
use super::fixtures::{Refusal, verify};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::executable_program::fusion::types::{Barrier, Fusibility};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::query_operators::types::OperationKind;

/// The classification the coverage check is handed for the compiled corpus of every operation.
#[derive(Clone, Copy, Debug)]
enum Classified {
    /// The program's own classification.
    AsCompiled,
    /// A Join instruction left unclassified.
    JoinUnclassified,
    /// A Rows instruction relabelled as fusable.
    RowsRelabelledFusable,
    /// The fused continuation relabelled as a shared-consumer barrier.
    FusedRelabelledBarrier,
}

type Input = Classified;
/// A verified classification answers nothing more.
type Value = ();
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_operation_kind_and_route_has_a_fusion_classification"),
        word: Word::Succeeds,
        input: || Classified::AsCompiled,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_unclassified_instruction_is_refused"),
        word: Word::Refuses,
        input: || Classified::JoinUnclassified,
        expect: Expect::Error(Refusal::Unclassified),
    },
    Case {
        name: CaseName("a_kind_relabelled_as_fusable_is_refused"),
        word: Word::Refuses,
        input: || Classified::RowsRelabelledFusable,
        expect: Expect::Error(Refusal::Mismatch),
    },
    Case {
        name: CaseName("a_fused_continuation_relabelled_as_a_barrier_is_refused"),
        word: Word::Refuses,
        input: || Classified::FusedRelabelledBarrier,
        expect: Expect::Error(Refusal::Mismatch),
    },
];

/// Verifies the corpus against the classification.
fn subject(input: Input) -> Result<Value, Failure> {
    let corpus = compiled_operations();
    let first_of = |kind| {
        corpus
            .program
            .regions()
            .iter()
            .flat_map(|region| region.instructions().iter())
            .find(|instruction| kind_of(&instruction.operation()) == kind)
            .map(|instruction| instruction.logical())
    };
    let changed = match input {
        Classified::AsCompiled => None,
        Classified::JoinUnclassified => first_of(OperationKind::Join).map(|at| (at, None)),
        Classified::RowsRelabelledFusable => {
            first_of(OperationKind::Rows).map(|at| (at, Some(Fusibility::Fusable)))
        }
        Classified::FusedRelabelledBarrier => corpus
            .program
            .regions()
            .iter()
            .find(|region| region.instructions().len() > 1)
            .map(|region| {
                (
                    region.instructions()[1].logical(),
                    Some(Fusibility::Barrier(Barrier::SharedConsumer)),
                )
            }),
    };
    let lookup = |logical| match changed {
        Some((at, label)) if at == logical => label,
        _ => corpus.program.fusibility(logical),
    };
    verify(&corpus, &lookup)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
