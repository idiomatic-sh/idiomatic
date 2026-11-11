//! The engine's sharing contract, language-free: every admitted sharing class is shared by two
//! rules and walks one traversal beside a distinct class, and every barrier kind is accounted
//! for. Coverage is an exhaustive match over the engine's own enums, so a new class shape,
//! partition or barrier fails to compile until it has a case here.
use super::fixtures::{
    accounted, admitted_head, after_barrier, after_scope, barrier_fixture, counted, program, shares,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::OperationKind;

fn every_admitted_class_is_shared_across_rules_in_one_traversal() {
    let mut scope = after_scope(None);
    let mut cases = 0;
    while let Some(within) = scope {
        for kind in OperationKind::ALL.iter().copied() {
            if let Some(head) = admitted_head(kind) {
                shares(within, head);
                cases += 1;
            }
        }
        scope = after_scope(Some(within));
    }
    assert!(cases > 0, "at least one admitted class shape");
}

fn every_barrier_kind_is_accounted_for() {
    let mut barrier = after_barrier(None);
    while let Some(kind) = barrier {
        let census = accounted(&program(barrier_fixture(kind)));
        assert!(
            counted(&census, kind) > 0,
            "the {kind:?} fixture holds an instruction behind its barrier"
        );
        barrier = after_barrier(Some(kind));
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    EveryAdmittedClassIsSharedAcrossRulesInOneTraversal,
    EveryBarrierKindIsAccountedFor,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_admitted_class_is_shared_across_rules_in_one_traversal"),
        word: Word::Succeeds,
        input: || Scenario::EveryAdmittedClassIsSharedAcrossRulesInOneTraversal,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("every_barrier_kind_is_accounted_for"),
        word: Word::Succeeds,
        input: || Scenario::EveryBarrierKindIsAccountedFor,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::EveryAdmittedClassIsSharedAcrossRulesInOneTraversal => {
            every_admitted_class_is_shared_across_rules_in_one_traversal();
            Ok(())
        }
        Scenario::EveryBarrierKindIsAccountedFor => {
            every_barrier_kind_is_accounted_for();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
