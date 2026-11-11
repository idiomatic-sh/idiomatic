//! One cell per family and mismatch: the issued capability binds the owner generation, the
//! exact plan, the admitted population and the issued descriptor.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Scenario, exercise,
};

fn every_family_refuses_every_mismatched_capability() {
    let refusals = [
        (Scenario::Stale, OperatingError::NotRead),
        (Scenario::OtherPlan, OperatingError::WrongOperation),
        (Scenario::OtherPopulation, OperatingError::NotRead),
        (
            Scenario::RetargetedDescriptor,
            OperatingError::WrongOperation,
        ),
    ];
    for kind in DerivedKind::ALL.iter().copied() {
        for (scenario, expected) in &refusals {
            assert_eq!(
                exercise(kind, *scenario).err().as_ref(),
                Some(expected),
                "{kind:?} {scenario:?}"
            );
        }
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Proof {
    EveryFamilyRefusesEveryMismatchedCapability,
}

type Input = Proof;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_family_refuses_every_mismatched_capability"),
    word: Word::Succeeds,
    input: || Proof::EveryFamilyRefusesEveryMismatchedCapability,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Proof::EveryFamilyRefusesEveryMismatchedCapability => {
            every_family_refuses_every_mismatched_capability();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
